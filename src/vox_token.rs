// Vox Token 类型定义
// v0.1 最小子集

/// 所有 Token 的种类
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // --- 关键字 ---
    Fn,
    While,
    Let,
    New,
    Mut,
    Return,
    If,
    Else,
    True,
    False,
    Match,

    // --- 类型关键字 ---
    I32,
    Bool,
    Str,
    Void,
    F64,
    Struct,
    Enum,

    // --- 字面量 ---
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    Identifier(String),

    // --- 运算符 ---
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Bang,      // !
    Eq,        // =
    EqEq,      // ==
    NotEq,     // !=
    Lt,        // <
    Gt,        // >
    LtEq,      // <=
    GtEq,      // >=
    Ampersand, // &
    AndAnd,    // &&
    PipePipe,  // ||
    Arrow,     // ->

    // --- 分隔符 ---
    LParen,    // (
    RParen,    // )
    LBracket,  // [
    RBracket,  // ]
    LBrace,    // {
    RBrace,    // }
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    Dot,       // .

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
