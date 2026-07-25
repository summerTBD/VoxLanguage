// Vox Language Server (LSP) — JSON-RPC over stdin/stdout
// 调 voxc --check 做检查，保证和编译器 100% 一致

use lsp_server::{Connection, Message};
use lsp_types::*;

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

    let base_dir = {
        let s = uri.as_str();
        if let Some(path) = s.strip_prefix("file:///") {
            // URL 解码 %3A → : 等
            let decoded = path.replace("%3A", ":").replace("%3a", ":");
            let path = decoded.rsplit_once('/').map(|(d, _)| d).unwrap_or(".");
            std::path::Path::new(&*path).to_path_buf()
        } else {
            std::path::Path::new(".").to_path_buf()
        }
    };

    let diags = check(&text, &base_dir);
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

fn check(source: &str, base_dir: &std::path::Path) -> Vec<Diagnostic> {
    let tmp = base_dir.join(format!("_vox_lsp_{}.vox", std::process::id()));
    std::fs::write(&tmp, source).ok();

    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| {
            let dir = p.parent().unwrap_or(std::path::Path::new("."));
            let candidates = [dir.join("voxc.exe"), dir.join("vox-language.exe")];
            candidates.into_iter().find(|c| c.exists())
        })
        .unwrap_or_else(|| std::path::Path::new("voxc.exe").to_path_buf());

    eprintln!(
        "Vox LSP check: exe={}, tmp={}",
        exe.display(),
        tmp.display()
    );

    let output = std::process::Command::new(&exe)
        .arg(&tmp)
        .arg("--check")
        .output();

    let _ = std::fs::remove_file(&tmp);

    match output {
        Ok(o) => {
            eprintln!(
                "Vox LSP: exit={}, stderr={}",
                o.status,
                String::from_utf8_lossy(&o.stderr)
            );
            if o.status.success() {
                vec![]
            } else {
                parse_diagnostics(&String::from_utf8_lossy(&o.stderr))
            }
        }
        Err(e) => {
            eprintln!("Vox LSP: spawn failed: {}", e);
            vec![]
        }
    }
}

fn parse_diagnostics(stderr: &str) -> Vec<Diagnostic> {
    stderr
        .lines()
        .filter_map(|line| {
            let (line_num, col, msg) = parse_loc(line);
            Some(Diagnostic {
                range: Range {
                    start: Position::new(line_num.saturating_sub(1), col.saturating_sub(1)),
                    end: Position::new(line_num.saturating_sub(1), col + 30),
                },
                severity: Some(DiagnosticSeverity::ERROR),
                message: msg,
                ..Default::default()
            })
        })
        .collect()
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
