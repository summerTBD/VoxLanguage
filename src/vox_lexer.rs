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
                    TokenKind::Minus
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '/' => {
                    self.advance();
                    TokenKind::Slash
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
                        // 预留 & 引用运算符，暂不支持就报错
                        panic!(
                            "词法错误: 第{}行第{}列: 不支持的字符 '&'（需要双写 &&）",
                            line, col
                        );
                    }
                }
                '|' => {
                    self.advance();
                    if self.match_char('|') {
                        TokenKind::PipePipe
                    } else {
                        panic!(
                            "词法错误: 第{}行第{}列: 不支持的字符 '|'（需要双写 ||）",
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
                    panic!("词法错误: 第{}行第{}列: 不识别的字符 '{}'", line, col, ch);
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
                    "词法错误: 第{}行第{}列: 字符串没有闭合",
                    self.line, self.col
                ),
                Some('"') => {
                    self.advance(); // 跳过结尾的 "
                    break;
                }
                Some('\\') => {
                    self.advance(); // 跳过反斜杠
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some(c) => {
                            // 不识别的转义，原样保留
                            s.push('\\');
                            s.push(c);
                        }
                        None => panic!("词法错误: 字符串中反斜杠后没有字符"),
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

    /// 读取整数 42
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
        let value: i64 = num_str.parse().expect("词法错误: 无法解析整数");
        TokenKind::IntLiteral(value)
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
            "!" => TokenKind::Bang,
            // 类型关键字
            "i32" => TokenKind::I32,
            "bool" => TokenKind::Bool,
            "string" => TokenKind::String,
            "void" => TokenKind::Void,
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
