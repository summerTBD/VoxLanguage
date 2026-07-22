//第一层：程序结构

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_annot: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    String,
    Bool,
    Void,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub content: Vec<Statement>,
}

//第二层：语句

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        type_annot: Type,
        value: Box<Expression>,
        mutable: bool,
    },
    Return(Option<Box<Expression>>),
    Expr(Box<Expression>),
    If {
        condition: Box<Expression>,
        then_block: Block,
        else_block: Option<Block>,
    },
}

//第三层：表达式

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    IntLiteral(i64),
    StringLiteral(String),
    BoolLiteral(bool),
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>,
    },
    Call {
        name: String,
        args: Vec<Expression>,
    },
}
