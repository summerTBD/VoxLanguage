// Vox Language Server (LSP) — JSON-RPC over stdin/stdout

use lsp_server::{Connection, Message};
use lsp_types::*;
use std::panic::catch_unwind;
use vox_language::{vox_lexer::Lexer, vox_parser::Parser, vox_typeck::TypeChecker};

fn main() {
    eprintln!("Vox LSP 启动");
    let (conn, io_threads) = Connection::stdio();
    let caps = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    })
    .unwrap();
    conn.initialize(caps).unwrap();

    for msg in &conn.receiver {
        match msg {
            Message::Request(req) => {
                if conn.handle_shutdown(&req).unwrap() {
                    break;
                }
            }
            Message::Notification(n) => handle(&conn, &n),
            _ => {}
        }
    }
    io_threads.join().unwrap();
}

fn handle(conn: &Connection, n: &lsp_server::Notification) {
    let (uri, text) = match n.method.as_str() {
        "textDocument/didOpen" => {
            let p: DidOpenTextDocumentParams = serde_json::from_value(n.params.clone()).unwrap();
            (p.text_document.uri, p.text_document.text)
        }
        "textDocument/didChange" => {
            let p: DidChangeTextDocumentParams = serde_json::from_value(n.params.clone()).unwrap();
            let text = p
                .content_changes
                .into_iter()
                .last()
                .map(|c| c.text)
                .unwrap_or_default();
            (p.text_document.uri, text)
        }
        _ => return,
    };

    let diags = check(&text);
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: diags,
        version: None,
    };
    conn.sender
        .send(Message::Notification(lsp_server::Notification {
            method: "textDocument/publishDiagnostics".into(),
            params: serde_json::to_value(params).unwrap(),
        }))
        .ok();
}

fn check(source: &str) -> Vec<Diagnostic> {
    // 过滤 # 行并替换 #define 名称
    let mut define_names: Vec<(String, String)> = Vec::new();
    let mut expanded = source.to_string();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("#define ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                let name = parts[0];
                let value = parts[1].trim();
                expanded = replace_ident(&expanded, name, value);
                define_names.push((name.into(), value.into()));
            }
        }
    }
    // 移除所有 # 行
    let clean_source: String = expanded
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let prelude = include_str!("../../prelude.vox");
    let prelude_lines = prelude.lines().count() as u32;
    let full_source = format!("{}\n{}", prelude, clean_source);

    match catch_unwind(std::panic::AssertUnwindSafe(|| {
        let lexer = Lexer::new(&full_source);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program();
        let mut typeck = TypeChecker::new();
        typeck.register_defines(&define_names);
        typeck.check(&program);
    })) {
        Ok(()) => vec![],
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            let (line, col, msg) = parse_loc(&msg);
            // 减去 prelude 的行数偏移
            let line = line.saturating_sub(prelude_lines);
            vec![Diagnostic {
                range: Range {
                    start: Position::new(line.saturating_sub(1), col.saturating_sub(1)),
                    end: Position::new(line.saturating_sub(1), col + 30),
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: msg,
                ..Default::default()
            }]
        }
    }
}

fn parse_loc(msg: &str) -> (u32, u32, String) {
    for prefix in &["Syntax error: ", "Type error: ", "Lex error: "] {
        if let Some(rest) = msg.strip_prefix(prefix) {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix("line ") {
                if let Some((num, rest)) = rest.split_once(' ') {
                    let line: u32 = num.parse().unwrap_or(1);
                    if let Some(rest) = rest.strip_prefix("col ") {
                        if let Some((num, rest)) = rest.split_once(": ") {
                            return (line, num.parse().unwrap_or(1), rest.into());
                        }
                    }
                }
            }
            return (1, 1, rest.into());
        }
    }
    (1, 1, msg.into())
}

fn replace_ident(source: &str, name: &str, value: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let name_bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + name_bytes.len() <= bytes.len()
            && &bytes[i..i + name_bytes.len()] == name_bytes
            && (i == 0 || !is_ident_char(bytes[i - 1]))
            && (i + name_bytes.len() == bytes.len() || !is_ident_char(bytes[i + name_bytes.len()]))
        {
            result.push_str(value);
            i += name_bytes.len();
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
