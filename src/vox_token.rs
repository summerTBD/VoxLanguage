// Vox Token 类型定义
// v0.1 最小子集

/// 所有 Token 的种类
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // --- 关键字 ---
    Fn,
    While,
    Let,
    Mut,
    Return,
    If,
    Else,
    True,
    False,

    // --- 类型关键字 ---
    I32,
    Bool,
    String,
    Void,

    // --- 字面量 ---
    IntLiteral(i64),
    StringLiteral(String),
    Identifier(String),

    // --- 运算符 ---
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Bang,     // !
    Eq,       // =
    EqEq,     // ==
    NotEq,    // !=
    Lt,       // <
    Gt,       // >
    LtEq,     // <=
    GtEq,     // >=
    AndAnd,   // &&
    PipePipe, // ||

    // --- 分隔符 ---
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;

    // --- 其他 ---
    Eof,
}

/// 源码中的一个 Token，携带位置信息
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Token { kind, line, col }
    }
}
