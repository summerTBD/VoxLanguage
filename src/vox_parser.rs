use crate::{
    vox_ast::{
        BinOp, Block, EnumDef, EnumVariant, Expression, Function, MatchArm, Param, Program,
        Statement, StructDef, StructField, Type,
    },
    vox_lexer::Lexer,
    vox_token::{Token, TokenKind},
};

pub struct Parser {
    pub current: Token,
    pub lexer: Lexer,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Parser {
        let current = lexer.next_token();
        Parser { lexer, current }
    }

    pub fn peek(&self) -> Token {
        self.current.clone()
    }

    pub fn advance(&mut self) {
        self.current = self.lexer.next_token();
    }

    pub fn expect(&self, expected: TokenKind) {
        if self.current.kind != expected {
            panic!(
                "Syntax error: line {} col {}: expected {:?}, got {:?}",
                self.current.line, self.current.col, expected, self.current.kind
            );
        }
    }

    pub fn parse_stmt(&mut self) -> Statement {
        match &self.current.kind {
            TokenKind::Let => {
                self.advance(); // 吞掉 let

                // 可选的 mut
                let mutable = if self.current.kind == TokenKind::Mut {
                    self.advance();
                    true
                } else {
                    false
                };

                // 变量名
                let name = match &self.current.kind {
                    TokenKind::Identifier(s) => s.clone(),
                    _ => panic!(
                        "Syntax error: line {}: expected variable name after let, got {:?}",
                        self.current.line, self.current.kind
                    ),
                };
                self.advance(); // 吞掉变量名

                // :
                self.expect(TokenKind::Colon);
                self.advance();

                // 类型注解
                let type_annot = self.parse_type();
                // parse_type 内部已经 advance 了

                // =
                self.expect(TokenKind::Eq);
                self.advance();

                // 初始值表达式
                let value = self.parse_expr();
                // parse_expr 内部已经 advance 了

                // ;
                self.expect(TokenKind::Semicolon);
                self.advance();

                Statement::Let {
                    name,
                    type_annot,
                    value: Box::new(value),
                    mutable,
                }
            }
            TokenKind::Return => {
                self.advance(); // 吞掉 return
                let value = self.parse_expr();
                self.expect(TokenKind::Semicolon);
                self.advance();

                Statement::Return(Some(Box::new(value)))
            }
            TokenKind::If => {
                self.advance(); // 吞掉 if

                // 条件表达式
                let condition = Box::new(self.parse_expr());

                // { ... }
                self.expect(TokenKind::LBrace);
                self.advance();
                let then_block = self.parse_block();
                // parse_block 内部已跳过 }

                // 可选的 else
                let else_block = if self.current.kind == TokenKind::Else {
                    self.advance(); // 吞掉 else
                    self.expect(TokenKind::LBrace);
                    self.advance();
                    let block = self.parse_block();
                    Some(block)
                } else {
                    None
                };

                Statement::If {
                    condition,
                    then_block,
                    else_block,
                }
            }
            TokenKind::While => {
                self.advance(); // 吞掉 while

                // 条件表达式
                let condition = Box::new(self.parse_expr());

                // { ... }
                self.expect(TokenKind::LBrace);
                self.advance();
                let body = self.parse_block();

                Statement::While { condition, body }
            }

            TokenKind::Match => {
                self.advance(); // 吞掉 match
                let expr = self.parse_expr();
                self.expect(TokenKind::LBrace);
                self.advance();
                let arms = self.parse_match_arms();
                Statement::Match {
                    expr: Box::new(expr),
                    arms,
                }
            }

            TokenKind::For => {
                self.advance(); // 吞掉 for
                self.expect(TokenKind::LParen);
                self.advance(); // 吞掉 (

                // init（可选）：let 声明 或 表达式
                let init = if self.current.kind == TokenKind::Semicolon {
                    None
                } else if self.current.kind == TokenKind::Let {
                    let s = self.parse_stmt(); // Let 自带分号
                    Some(Box::new(s))
                } else {
                    let e = self.parse_expr();
                    // 表达式没有分号，需要手动吞掉
                    self.expect(TokenKind::Semicolon);
                    self.advance();
                    Some(Box::new(Statement::Expr(Box::new(e))))
                };

                // condition（可选）
                let condition = if self.current.kind == TokenKind::Semicolon {
                    None
                } else {
                    let e = self.parse_expr();
                    Some(Box::new(e))
                };
                self.expect(TokenKind::Semicolon);
                self.advance(); // 吞掉 ;

                // step（可选）：表达式或赋值
                let step = if self.current.kind == TokenKind::RParen {
                    None
                } else {
                    let expr = self.parse_expr();
                    if self.current.kind == TokenKind::Eq {
                        // i = i + 1 这种赋值
                        self.advance(); // 吞掉 =
                        let value = self.parse_expr();
                        let name = match expr {
                            Expression::Identifier(n) => n,
                            _ => panic!(
                                "Syntax error: line {}: cannot assign to this in for-step",
                                self.current.line
                            ),
                        };
                        Some(Box::new(Statement::Assign {
                            name,
                            value: Box::new(value),
                        }))
                    } else {
                        Some(Box::new(Statement::Expr(Box::new(expr))))
                    }
                };
                self.expect(TokenKind::RParen);
                self.advance(); // 吞掉 )

                // body
                self.expect(TokenKind::LBrace);
                self.advance();
                let body = self.parse_block();

                Statement::For {
                    init,
                    condition,
                    step,
                    body,
                }
            }

            TokenKind::Break => {
                self.advance();
                self.expect(TokenKind::Semicolon);
                self.advance();
                Statement::Break
            }

            TokenKind::Continue => {
                self.advance();
                self.expect(TokenKind::Semicolon);
                self.advance();
                Statement::Continue
            }

            // 表达式开头：赋值 x = expr;  调用 print(args);  运算 a + b;
            // 注：parse_expr 会处理标识符、*, !, & 等多种开头
            _ => {
                let expr = self.parse_expr();

                if self.current.kind == TokenKind::Eq {
                    // x = expr  或  *p = expr
                    self.advance(); // 吞掉 =
                    let value = Box::new(self.parse_expr());
                    self.expect(TokenKind::Semicolon);
                    self.advance();

                    return match expr {
                        Expression::Identifier(name) => Statement::Assign { name, value },
                        Expression::Deref(inner) => Statement::Store { ptr: inner, value },
                        Expression::FieldAccess { object, field } => Statement::StoreField {
                            object,
                            field,
                            value,
                        },
                        Expression::Index { array, index } => Statement::StoreIndex {
                            array,
                            index,
                            value,
                        },
                        _ => panic!(
                            "Syntax error: line {}: cannot assign to this expression",
                            self.current.line
                        ),
                    };
                }

                // 表达式语句：a + b;  print(x);  *p;
                self.expect(TokenKind::Semicolon);
                self.advance();
                Statement::Expr(Box::new(expr))
            }
        }
    }

    pub fn parse_type(&mut self) -> Type {
        // 递归处理 * 前缀：*i32, **i32, *MyStruct
        if self.current.kind == TokenKind::Star {
            self.advance(); // 吞掉 *
            let inner = self.parse_type();
            return Type::Ptr(Box::new(inner));
        }

        // [T; N]  数组类型
        if self.current.kind == TokenKind::LBracket {
            self.advance(); // 吞掉 [
            let elem_ty = self.parse_type();
            self.expect(TokenKind::Semicolon);
            self.advance(); // 吞掉 ;
            let size = match &self.current.kind {
                TokenKind::IntLiteral(n) => *n as usize,
                _ => panic!(
                    "Syntax error: line {}: expected array size (integer), got {:?}",
                    self.current.line, self.current.kind
                ),
            };
            self.advance(); // 吞掉数字
            self.expect(TokenKind::RBracket);
            self.advance(); // 吞掉 ]
            return Type::Array(Box::new(elem_ty), size);
        }

        let ty = match &self.current.kind {
            TokenKind::KwI8 => {
                self.advance();
                Type::Int(crate::vox_ast::Signedness::Signed, 8)
            }
            TokenKind::KwI16 => {
                self.advance();
                Type::Int(crate::vox_ast::Signedness::Signed, 16)
            }
            TokenKind::KwI32 => {
                self.advance();
                Type::Int(crate::vox_ast::Signedness::Signed, 32)
            }
            TokenKind::KwI64 => {
                self.advance();
                Type::Int(crate::vox_ast::Signedness::Signed, 64)
            }
            TokenKind::KwU8 => {
                self.advance();
                Type::Int(crate::vox_ast::Signedness::Unsigned, 8)
            }
            TokenKind::KwU16 => {
                self.advance();
                Type::Int(crate::vox_ast::Signedness::Unsigned, 16)
            }
            TokenKind::KwU32 => {
                self.advance();
                Type::Int(crate::vox_ast::Signedness::Unsigned, 32)
            }
            TokenKind::KwU64 => {
                self.advance();
                Type::Int(crate::vox_ast::Signedness::Unsigned, 64)
            }
            TokenKind::KwF32 => {
                self.advance();
                Type::Float(32)
            }
            TokenKind::KwF64 => {
                self.advance();
                Type::Float(64)
            }
            TokenKind::KwChar => {
                self.advance();
                Type::Char
            }
            TokenKind::KwBool => {
                self.advance();
                Type::Bool
            }
            TokenKind::KwStr => {
                self.advance();
                Type::Str
            }
            TokenKind::KwVoid => {
                self.advance();
                Type::Void
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance(); // 吞掉标识符

                // 类型参数：<i32, Str>
                let args = if self.current.kind == TokenKind::Lt {
                    self.advance(); // 吞掉 <
                    let args = self.parse_type_args();
                    self.expect(TokenKind::Gt);
                    self.advance(); // 吞掉 >
                    args
                } else {
                    vec![]
                };

                return Type::Adt { name, args };
            }
            _ => panic!(
                "Syntax error: line {}: expected type (i32/bool/str/void or struct name), got {:?}",
                self.current.line, self.current.kind
            ),
        };
        ty
    }

    // ==================== 表达式解析 ====================

    /// 解析类型参数列表：<i32, Str, *Point>
    fn parse_type_args(&mut self) -> Vec<Type> {
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type());
            if self.current.kind == TokenKind::Comma {
                self.advance();
            } else {
                break;
            }
        }
        args
    }

    /// 入口：比较运算（最低优先级）
    pub fn parse_expr(&mut self) -> Expression {
        self.parse_comparison()
    }

    /// == != < > <= >=
    fn parse_comparison(&mut self) -> Expression {
        let mut left = self.parse_addition();

        loop {
            let op = match self.current.kind {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::NotEq => BinOp::NotEq,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_addition();
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        left
    }

    /// + -
    fn parse_addition(&mut self) -> Expression {
        let mut left = self.parse_multiplication();

        loop {
            let op = match self.current.kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication();
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        left
    }

    /// * /
    fn parse_multiplication(&mut self) -> Expression {
        let mut left = self.parse_postfix();

        loop {
            let op = match self.current.kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_postfix();
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        left
    }

    /// 原子：字面量、标识符、函数调用、括号
    fn parse_primary(&mut self) -> Expression {
        match &self.current.kind {
            TokenKind::Bang => {
                self.advance(); // 吞掉 !
                let inner = self.parse_postfix();
                Expression::Not(Box::new(inner))
            }
            TokenKind::Sizeof => {
                self.advance(); // 吞掉 sizeof
                self.expect(TokenKind::LParen);
                self.advance(); // 吞掉 (
                let ty = self.parse_type();
                self.expect(TokenKind::RParen);
                self.advance(); // 吞掉 )
                Expression::Sizeof(ty)
            }
            TokenKind::New => {
                self.advance(); // 吞掉 new
                let name = match &self.current.kind {
                    TokenKind::Identifier(s) => s.clone(),
                    _ => panic!(
                        "Syntax error: line {}: expected struct name after new",
                        self.current.line
                    ),
                };
                self.advance();

                // 跳过 <T> 类型参数（保留语法，以后泛型用）
                if self.current.kind == TokenKind::Lt {
                    self.advance(); // 吞掉 <
                    let _args = self.parse_type_args();
                    self.expect(TokenKind::Gt);
                    self.advance(); // 吞掉 >
                }

                self.expect(TokenKind::LBrace);
                self.advance();
                let mut fields = Vec::new();
                while self.current.kind != TokenKind::RBrace {
                    let fname = match &self.current.kind {
                        TokenKind::Identifier(s) => s.clone(),
                        _ => panic!(
                            "Syntax error: line {}: expected field name",
                            self.current.line
                        ),
                    };
                    self.advance();
                    self.expect(TokenKind::Colon);
                    self.advance();
                    let val = self.parse_expr();
                    fields.push((fname, val));
                    if self.current.kind == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBrace);
                self.advance();
                Expression::New { name, fields }
            }
            TokenKind::IntLiteral(n) => {
                let val = *n;
                self.advance();
                Expression::IntLiteral(val)
            }
            TokenKind::FloatLiteral(n) => {
                let val = *n;
                self.advance();
                Expression::FloatLiteral(val)
            }
            TokenKind::StringLiteral(s) => {
                let val = s.clone();
                self.advance();
                Expression::StringLiteral(val)
            }
            TokenKind::True => {
                self.advance();
                Expression::BoolLiteral(true)
            }
            TokenKind::False => {
                self.advance();
                Expression::BoolLiteral(false)
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();

                // 函数调用：foo(args)
                if self.current.kind == TokenKind::LParen {
                    self.advance(); // 跳过 (
                    let mut args = Vec::new();

                    if self.current.kind != TokenKind::RParen {
                        loop {
                            args.push(self.parse_expr());
                            if self.current.kind == TokenKind::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }

                    self.expect(TokenKind::RParen);
                    self.advance(); // 跳过 )

                    Expression::Call { name, args }
                } else if self.current.kind == TokenKind::LBrace
                    && name.chars().next().map_or(false, |c| c.is_uppercase())
                {
                    // 结构体字面量：Point { x: 10, y: 20 }（仅大写开头触发）
                    self.advance();
                    let mut fields = Vec::new();
                    while self.current.kind != TokenKind::RBrace {
                        let field_name = match &self.current.kind {
                            TokenKind::Identifier(s) => s.clone(),
                            _ => panic!("语法错误: 第{}行: 期望字段名", self.current.line),
                        };
                        self.advance();
                        self.expect(TokenKind::Colon);
                        self.advance();
                        let value = self.parse_expr();
                        fields.push((field_name, value));
                        if self.current.kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace);
                    self.advance();
                    Expression::StructLiteral { name, fields }
                } else {
                    Expression::Identifier(name)
                }
            }
            TokenKind::LParen => {
                self.advance(); // 跳过 (
                let expr = self.parse_expr();
                self.expect(TokenKind::RParen);
                self.advance(); // 跳过 )
                expr
            }

            // &expr  取地址（一元前缀运算符，优先级同解引用）
            TokenKind::Ampersand => {
                self.advance();
                let expr = self.parse_postfix();
                Expression::AddrOf(Box::new(expr))
            }

            // *expr  解引用（一元前缀运算符，优先级同取地址）
            TokenKind::Star => {
                self.advance();
                let expr = self.parse_postfix();
                Expression::Deref(Box::new(expr))
            }

            // [1, 2, 3]  数组字面量
            TokenKind::LBracket => {
                self.advance(); // 吞掉 [
                let mut elements = Vec::new();
                if self.current.kind != TokenKind::RBracket {
                    loop {
                        elements.push(self.parse_expr());
                        if self.current.kind == TokenKind::Comma {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket);
                self.advance(); // 吞掉 ]
                Expression::ArrayLiteral(elements)
            }

            _ => panic!(
                "Syntax error: line {}: expected expression, got {:?}",
                self.current.line, self.current.kind
            ),
        }
    }

    /// 后缀操作：p.x 字段访问, arr[i] 数组下标
    fn parse_postfix(&mut self) -> Expression {
        let mut expr = self.parse_primary();

        loop {
            if self.current.kind == TokenKind::Dot {
                self.advance(); // 跳过 .
                let field = match &self.current.kind {
                    TokenKind::Identifier(s) => s.clone(),
                    _ => panic!("语法错误: 第{}行: 期望字段名", self.current.line),
                };
                self.advance();
                expr = Expression::FieldAccess {
                    object: Box::new(expr),
                    field,
                };
            } else if self.current.kind == TokenKind::LBracket {
                self.advance(); // 跳过 [
                let index = self.parse_expr();
                self.expect(TokenKind::RBracket);
                self.advance(); // 跳过 ]
                expr = Expression::Index {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.current.kind == TokenKind::As {
                self.advance();
                let target = self.parse_type();
                expr = Expression::Cast {
                    expr: Box::new(expr),
                    target,
                };
            } else {
                break;
            }
        }

        expr
    }

    // ==================== 块解析 ====================

    pub fn parse_block(&mut self) -> Block {
        let mut stmts = Vec::new();

        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::Eof {
            stmts.push(self.parse_stmt());
        }

        self.expect(TokenKind::RBrace);
        self.advance(); // 跳过 }

        Block { content: stmts }
    }

    fn parse_match_arms(&mut self) -> Vec<MatchArm> {
        let mut arms = Vec::new();

        while self.current.kind != TokenKind::RBrace && self.current.kind != TokenKind::Eof {
            // pattern（变体名）
            let pattern = match &self.current.kind {
                TokenKind::Identifier(s) => s.clone(),
                _ => panic!(
                    "Syntax error: line {}: expected match pattern (variant name), got {:?}",
                    self.current.line, self.current.kind
                ),
            };
            self.advance();

            // ->
            self.expect(TokenKind::Arrow);
            self.advance();

            // { body }
            self.expect(TokenKind::LBrace);
            self.advance();
            let body = self.parse_block();
            // parse_block 已消费 }

            arms.push(MatchArm { pattern, body });
        }

        self.expect(TokenKind::RBrace);
        self.advance(); // 跳过 match 的 }

        arms
    }

    // ==================== 函数与程序解析 ====================

    pub fn parse_program(&mut self) -> Program {
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut consts = Vec::new();
        let mut statics = Vec::new();

        while self.current.kind != TokenKind::Eof {
            if self.current.kind == TokenKind::Fn {
                functions.push(self.parse_function(false));
            } else if self.current.kind == TokenKind::Extern {
                functions.push(self.parse_function(true));
            } else if self.current.kind == TokenKind::KwStruct {
                structs.push(self.parse_struct_def());
            } else if self.current.kind == TokenKind::KwEnum {
                enums.push(self.parse_enum_def());
            } else if self.current.kind == TokenKind::Const {
                consts.push(self.parse_const_def());
            } else if self.current.kind == TokenKind::Static {
                statics.push(self.parse_static_def());
            } else {
                panic!(
                    "Syntax error: line {}: expected fn/struct/enum/const/static, got {:?}",
                    self.current.line, self.current.kind
                );
            }
        }

        Program {
            functions,
            structs,
            enums,
            consts,
            statics,
        }
    }

    pub fn parse_struct_def(&mut self) -> StructDef {
        self.expect(TokenKind::KwStruct);
        self.advance(); // 跳过 struct

        // 结构体名
        let name = match &self.current.kind {
            TokenKind::Identifier(s) => s.clone(),
            _ => panic!(
                "Syntax error: line {}: expected struct name",
                self.current.line
            ),
        };
        self.advance();

        // { 字段 }
        self.expect(TokenKind::LBrace);
        self.advance();

        let mut fields = Vec::new();
        while self.current.kind != TokenKind::RBrace {
            let field_name = match &self.current.kind {
                TokenKind::Identifier(s) => s.clone(),
                _ => panic!("语法错误: 第{}行: 期望字段名", self.current.line),
            };
            self.advance();

            self.expect(TokenKind::Colon);
            self.advance();

            let type_annot = self.parse_type(); //会往前移动一个Token

            fields.push(StructField {
                name: field_name,
                type_annot,
            });

            if self.current.kind == TokenKind::Comma {
                self.advance();
            }
        }

        self.expect(TokenKind::RBrace);
        self.advance();

        StructDef { name, fields }
    }

    pub fn parse_enum_def(&mut self) -> EnumDef {
        self.expect(TokenKind::KwEnum);
        self.advance(); // 跳过 enum

        // 枚举名
        let name = match &self.current.kind {
            TokenKind::Identifier(s) => s.clone(),
            _ => panic!(
                "Syntax error: line {}: expected enum name",
                self.current.line
            ),
        };
        self.advance();

        // { 变体 }
        self.expect(TokenKind::LBrace);
        self.advance();

        let mut variants = Vec::new();
        let mut disc = 0i32;
        while self.current.kind != TokenKind::RBrace {
            let variant_name = match &self.current.kind {
                TokenKind::Identifier(s) => s.clone(),
                _ => panic!("语法错误: 第{}行: 期望枚举变体名", self.current.line),
            };
            self.advance();

            // 可选 = 判别值
            let discriminant = if self.current.kind == TokenKind::Eq {
                self.advance();
                match &self.current.kind {
                    TokenKind::IntLiteral(n) => {
                        let val = *n as i32;
                        self.advance();
                        disc = val;
                        val
                    }
                    _ => panic!("语法错误: 第{}行: 期望整数判别值", self.current.line),
                }
            } else {
                let val = disc;
                disc += 1;
                val
            };

            variants.push(EnumVariant {
                name: variant_name,
                discriminant,
            });

            if self.current.kind == TokenKind::Comma {
                self.advance();
            }
        }

        self.expect(TokenKind::RBrace);
        self.advance();

        EnumDef { name, variants }
    }

    pub fn parse_const_def(&mut self) -> crate::vox_ast::ConstDef {
        self.expect(TokenKind::Const);
        self.advance(); // 跳过 const

        let name = match &self.current.kind {
            TokenKind::Identifier(s) => s.clone(),
            _ => panic!(
                "Syntax error: line {}: expected const name",
                self.current.line
            ),
        };
        self.advance();

        self.expect(TokenKind::Colon);
        self.advance();

        let type_annot = self.parse_type();

        self.expect(TokenKind::Eq);
        self.advance();

        let value = self.parse_expr();

        self.expect(TokenKind::Semicolon);
        self.advance();

        crate::vox_ast::ConstDef {
            name,
            type_annot,
            value,
        }
    }

    pub fn parse_static_def(&mut self) -> crate::vox_ast::StaticDef {
        self.expect(TokenKind::Static);
        self.advance(); // 跳过 static

        let mutable = if self.current.kind == TokenKind::Mut {
            self.advance();
            true
        } else {
            false
        };

        let name = match &self.current.kind {
            TokenKind::Identifier(s) => s.clone(),
            _ => panic!(
                "Syntax error: line {}: expected static name",
                self.current.line
            ),
        };
        self.advance();

        self.expect(TokenKind::Colon);
        self.advance();

        let type_annot = self.parse_type();

        self.expect(TokenKind::Eq);
        self.advance();

        let value = self.parse_expr();

        self.expect(TokenKind::Semicolon);
        self.advance();

        crate::vox_ast::StaticDef {
            name,
            type_annot,
            value,
            mutable,
        }
    }

    pub fn parse_function(&mut self, is_extern: bool) -> Function {
        if is_extern {
            self.expect(TokenKind::Extern);
            self.advance(); // 跳过 extern
        }
        self.expect(TokenKind::Fn);
        self.advance(); // 跳过 fn

        // 函数名
        let name = match &self.current.kind {
            TokenKind::Identifier(s) => s.clone(),
            _ => panic!(
                "Syntax error: line {}: expected function name after fn, got {:?}",
                self.current.line, self.current.kind
            ),
        };
        self.advance();

        // ( 参数列表 )
        self.expect(TokenKind::LParen);
        self.advance();
        let params = self.parse_params();
        self.expect(TokenKind::RParen);
        self.advance();

        // 可选的返回类型
        let return_type = if self.current.kind == TokenKind::Colon {
            self.advance(); // 跳过 :
            self.parse_type()
        } else {
            Type::Void
        };

        // extern 声明：分号结尾，无函数体
        let body = if is_extern {
            self.expect(TokenKind::Semicolon);
            self.advance();
            crate::vox_ast::Block { content: vec![] }
        } else {
            self.expect(TokenKind::LBrace);
            self.advance();
            self.parse_block()
        };

        Function {
            name,
            params,
            return_type,
            body,
            is_extern,
        }
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();

        // 无参数
        if self.current.kind == TokenKind::RParen {
            return params;
        }

        loop {
            // 参数名
            let name = match &self.current.kind {
                TokenKind::Identifier(s) => s.clone(),
                _ => panic!(
                    "Syntax error: line {}: expected parameter name, got {:?}",
                    self.current.line, self.current.kind
                ),
            };
            self.advance();

            // :
            self.expect(TokenKind::Colon);
            self.advance();

            // 类型
            let type_annot = self.parse_type();

            params.push(Param { name, type_annot });

            // 逗号 → 继续下一个参数；) → 结束
            if self.current.kind == TokenKind::Comma {
                self.advance();
                // 尾逗号处理：逗号后就是 )，允许
                if self.current.kind == TokenKind::RParen {
                    break;
                }
            } else {
                break;
            }
        }

        params
    }
}
