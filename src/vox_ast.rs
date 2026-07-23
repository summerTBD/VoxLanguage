//第一层：程序结构

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
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
    Str,
    Bool,
    Void,
    F64,
    Ptr(Box<Type>),                        // *i32 **i32
    Array(Box<Type>, usize),               // [i32; 10]
    Adt { name: String, args: Vec<Type> }, // Point, Color, Vec<i32>
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub content: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_annot: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub discriminant: i32,
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
    While {
        condition: Box<Expression>,
        body: Block,
    },
    Assign {
        name: String,
        value: Box<Expression>,
    },
    Match {
        expr: Box<Expression>,
        arms: Vec<MatchArm>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: String, // 变体名
    pub body: Block,
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
    FloatLiteral(f64),
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
    // 字段/变体访问
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },
    Not(Box<Expression>),
    New {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    AddrOf(Box<Expression>),
    Deref(Box<Expression>),
    ArrayLiteral(Vec<Expression>),
    Index {
        array: Box<Expression>,
        index: Box<Expression>,
    },
}
