//第一层：程序结构

#[derive(Debug, Clone, PartialEq)]
pub enum TopLevelItem {
    CppLine(String),
    Struct(StructDef),
    Enum(EnumDef),
    Function(Function),
    Const(ConstDef),
    Static(StaticDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<TopLevelItem>,
}

impl Program {
    pub fn functions(&self) -> Vec<&Function> {
        self.items.iter().filter_map(|i| if let TopLevelItem::Function(f) = i { Some(f) } else { None }).collect()
    }
    pub fn structs(&self) -> Vec<&StructDef> {
        self.items.iter().filter_map(|i| if let TopLevelItem::Struct(s) = i { Some(s) } else { None }).collect()
    }
    pub fn enums(&self) -> Vec<&EnumDef> {
        self.items.iter().filter_map(|i| if let TopLevelItem::Enum(e) = i { Some(e) } else { None }).collect()
    }
    pub fn consts(&self) -> Vec<&ConstDef> {
        self.items.iter().filter_map(|i| if let TopLevelItem::Const(c) = i { Some(c) } else { None }).collect()
    }
    pub fn statics(&self) -> Vec<&StaticDef> {
        self.items.iter().filter_map(|i| if let TopLevelItem::Static(s) = i { Some(s) } else { None }).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDef {
    pub name: String,
    pub type_annot: Type,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticDef {
    pub name: String,
    pub type_annot: Type,
    pub value: Expression,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Block,
    pub is_extern: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_annot: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Signedness {
    Signed,
    Unsigned,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int(Signedness, u8), // Int(Signed, 32) = i32
    Float(u8),           // Float(64) = f64
    Bool,
    Char,
    Str,
    Void,
    Ptr(Box<Type>),                        // *i32 **i32
    Array(Box<Type>, usize),               // [i32; 10]
    Adt { name: String, args: Vec<Type> }, // Point, Color
}

impl Type {
    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Int(_, _))
    }
    pub fn is_float(&self) -> bool {
        matches!(self, Type::Float(_))
    }
    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }
    /// 返回两类型中"更宽"的那个（整数按位宽，有符号<无符号）
    pub fn wider(&self, other: &Type) -> Type {
        match (self, other) {
            (Type::Int(s1, b1), Type::Int(s2, b2)) => {
                let bits = (*b1).max(*b2);
                let sign = match (s1, s2) {
                    (Signedness::Unsigned, _) | (_, Signedness::Unsigned) => Signedness::Unsigned,
                    _ => Signedness::Signed,
                };
                Type::Int(sign, bits)
            }
            (Type::Float(b1), Type::Float(b2)) => Type::Float((*b1).max(*b2)),
            _ => self.clone(),
        }
    }
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
    Store {
        ptr: Box<Expression>,
        value: Box<Expression>,
    },
    StoreField {
        object: Box<Expression>,
        field: String,
        value: Box<Expression>,
    },
    StoreIndex {
        array: Box<Expression>,
        index: Box<Expression>,
        value: Box<Expression>,
    },
    Match {
        expr: Box<Expression>,
        arms: Vec<MatchArm>,
    },
    For {
        init: Option<Box<Statement>>,
        condition: Option<Box<Expression>>,
        step: Option<Box<Statement>>,
        body: Block,
    },
    Break,
    Continue,
    Define {
        name: String,
        value: String,
    },
    CppDirective {
        directive: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: String, // 变体名
    pub body: Block,
}

//第三层：表达式

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
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
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
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
    Cast {
        expr: Box<Expression>,
        target: Type,
    },
    Sizeof(Type),
    Deref(Box<Expression>),
    ArrayLiteral(Vec<Expression>),
    Index {
        array: Box<Expression>,
        index: Box<Expression>,
    },
}
