// Vox 词法分析器 (Lexer)
// v0.1 最小子集

use crate::vox_token::{Token, TokenKind};

/// 词法分析器：将源码字符串转换为 Token 流
pub struct Lexer {
    /// 剩余待扫描的字符
    chars: Vec<char>,
    /// 当前位置索引
    pos: usize,
    /// 当前行号（从 1 开始）
    line: usize,
    /// 当前列号（从 1 开始）
    col: usize,
}

impl Lexer {
    /// 从源码字符串创建 Lexer
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// 收集所有 Token 并返回
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    // ========== 内部方法 ==========

    /// 查看当前字符但不消耗
    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// 消耗当前字符并返回
    pub fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    /// 如果当前字符符合预期则消耗，否则不做任何事
    pub fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// 跳过空白和注释
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                // 空白字符
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                    self.advance();
                }
                // 行注释 //
                Some('/') if self.peek_next() == Some('/') => {
                    self.advance(); // 跳过第一个 /
                    self.advance(); // 跳过第二个 /
                    while let Some(ch) = self.peek() {
                        if ch == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                // 块注释 /* */
                Some('/') if self.peek_next() == Some('*') => {
                    self.advance(); // 跳过 /
                    self.advance(); // 跳过 *
                    loop {
                        match self.peek() {
                            None => break, // 意外 EOF，容错
                            Some('*') if self.peek_next() == Some('/') => {
                                self.advance(); // 跳过 *
                                self.advance(); // 跳过 /
                                break;
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                }
                _ => break,
            }
        }
    }

    /// 查看下一个字符（不消耗）
    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    /// 读取下一个 Token
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let col = self.col;

        // # 预处理指令
        if self.peek() == Some('#') {
            return self.read_cpp_directive(line, col);
        }

        let kind = match self.peek() {
            None => TokenKind::Eof,
            Some(ch) => match ch {
                // --- 单字符分隔符 ---
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                '[' => {
                    self.advance();
                    TokenKind::LBracket
                }
                ']' => {
                    self.advance();
                    TokenKind::RBracket
                }
                '{' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    TokenKind::RBrace
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                ':' => {
                    self.advance();
                    TokenKind::Colon
                }
                '.' => {
                    self.advance();
                    TokenKind::Dot
                }
                ';' => {
                    self.advance();
                    TokenKind::Semicolon
                }

                // --- 运算符（单字符 + 双字符） ---
                '+' => {
                    self.advance();
                    TokenKind::Plus
                }
                '-' => {
                    self.advance();
                    if self.match_char('>') {
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '/' => {
                    self.advance();
                    TokenKind::Slash
                }
                '%' => {
                    self.advance();
                    TokenKind::Percent
                }
                '!' => {
                    self.advance();
                    if self.match_char('=') {
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }
                '=' => {
                    self.advance();
                    if self.match_char('=') {
                        TokenKind::EqEq
                    } else {
                        TokenKind::Eq
                    }
                }
                '<' => {
                    self.advance();
                    if self.match_char('=') {
                        TokenKind::LtEq
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.advance();
                    if self.match_char('=') {
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }
                '&' => {
                    self.advance();
                    if self.match_char('&') {
                        TokenKind::AndAnd
                    } else {
                        TokenKind::Ampersand
                    }
                }
                '|' => {
                    self.advance();
                    if self.match_char('|') {
                        TokenKind::PipePipe
                    } else {
                        panic!(
                            "Lex error: line {} col {}: '|' not supported (use ||)",
                            line, col
                        );
                    }
                }

                // --- 字符串字面量 ---
                '"' => self.read_string(),

                // --- 数字或标识符 ---
                c if c.is_ascii_digit() => self.read_number(),
                c if c.is_alphabetic() || c == '_' => self.read_identifier_or_keyword(),

                // --- 非法字符 ---
                _ => {
                    self.advance();
                    panic!(
                        "Lex error: line {} col {}: unknown char '{}'",
                        line, col, ch
                    );
                }
            },
        };

        Token::new(kind, line, col)
    }

    /// 读取字符串字面量 "hello"
    fn read_string(&mut self) -> TokenKind {
        self.advance(); // 跳过开头的 "
        let mut s = String::new();
        loop {
            match self.peek() {
                None => panic!(
                    "Lex error: line {} col {}: unclosed string",
                    self.line, self.col
                ),
                Some('"') => {
                    self.advance(); // 跳过结尾的 "
                    break;
                }
                Some('\\') => {
                    s.push('\\');
                    self.advance(); // 跳过反斜杠
                    if let Some(c) = self.peek() {
                        s.push(c);
                        self.advance();
                    }
                }
                Some(ch) => {
                    s.push(ch);
                    self.advance();
                }
            }
        }
        TokenKind::StringLiteral(s)
    }

    /// 读取 C 预处理指令：#define, #ifndef, #include 等
    fn read_cpp_directive(&mut self, line: usize, col: usize) -> Token {
        self.advance(); // 吞掉 #
        let kw = self.read_identifier(); // 读取关键字
        self.skip_whitespace_in_line();
        let rest = self.read_until_eol(); // 剩余部分

        let kind = match kw.as_str() {
            "define" => {
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts.len() >= 2 {
                    TokenKind::MacroDefine(parts[0].to_string(), parts[1].trim().to_string())
                } else {
                    TokenKind::MacroDefine(rest, String::new())
                }
            }
            "undef" => TokenKind::MacroUndef(rest),
            "include" => TokenKind::MacroInclude(rest),
            "ifdef" => TokenKind::MacroIfdef(rest),
            "ifndef" => TokenKind::MacroIfndef(rest),
            "if" => TokenKind::MacroIf(rest),
            "elif" => TokenKind::MacroElif(rest),
            "else" => TokenKind::MacroElse,
            "endif" => TokenKind::MacroEndif,
            "pragma" => TokenKind::MacroPragma(rest),
            "error" => TokenKind::MacroError(rest),
            "line" => TokenKind::MacroLine(rest),
            _ => TokenKind::MacroError(format!("#{} {}", kw, rest)),
        };
        Token::new(kind, line, col)
    }

    fn read_identifier(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn skip_whitespace_in_line(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_until_eol(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            s.push(ch);
            self.advance();
        }
        s.trim().to_string()
    }

    /// 读取整数或浮点数
    fn read_number(&mut self) -> TokenKind {
        let mut num_str = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        // 浮点数：数字后跟 .
        if self.peek() == Some('.') && self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
            num_str.push('.');
            self.advance(); // 跳过 .
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    num_str.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
            let value: f64 = num_str.parse().expect("Lex error: invalid float");
            TokenKind::FloatLiteral(value)
        } else {
            let value: i64 = num_str.parse().expect("Lex error: invalid integer");
            TokenKind::IntLiteral(value)
        }
    }

    /// 读取标识符或关键字
    fn read_identifier_or_keyword(&mut self) -> TokenKind {
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        // 匹配关键字
        match name.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "while" => TokenKind::While,
            "new" => TokenKind::New,
            "struct" => TokenKind::KwStruct,
            "enum" => TokenKind::KwEnum,
            "match" => TokenKind::Match,
            "extern" => TokenKind::Extern,
            "as" => TokenKind::As,
            "sizeof" => TokenKind::Sizeof,
            "for" => TokenKind::For,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "const" => TokenKind::Const,
            "static" => TokenKind::Static,
            "mod" => TokenKind::Mod,
            // 类型关键字
            "i8" => TokenKind::KwI8,
            "i16" => TokenKind::KwI16,
            "i32" => TokenKind::KwI32,
            "i64" => TokenKind::KwI64,
            "u8" => TokenKind::KwU8,
            "u16" => TokenKind::KwU16,
            "u32" => TokenKind::KwU32,
            "u64" => TokenKind::KwU64,
            "f32" => TokenKind::KwF32,
            "f64" => TokenKind::KwF64,
            "char" => TokenKind::KwChar,
            "bool" => TokenKind::KwBool,
            "str" => TokenKind::KwStr,
            "void" => TokenKind::KwVoid,
            _ => TokenKind::Identifier(name),
        }
    }
}

// ========== 测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("fn main() { }");
        let tokens = lexer.tokenize();
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Fn,
                TokenKind::Identifier("main".into()),
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_number_and_string() {
        let mut lexer = Lexer::new(r#"42 "hello""#);
        let tokens = lexer.tokenize();
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::IntLiteral(42),
                TokenKind::StringLiteral("hello".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("== != <= >= && ||");
        let tokens = lexer.tokenize();
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::AndAnd,
                TokenKind::PipePipe,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("fn let return if else true false");
        let tokens = lexer.tokenize();
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Fn,
                TokenKind::Let,
                TokenKind::Return,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::True,
                TokenKind::False,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("// 这是注释\n42 /* 块注释 */ 10");
        let tokens = lexer.tokenize();
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::IntLiteral(42),
                TokenKind::IntLiteral(10),
                TokenKind::Eof,
            ]
        );
    }
}
