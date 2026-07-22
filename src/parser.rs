use crate::{
    ast::{BinOp, Block, Expression, Function, Param, Program, Statement, Type},
    lexer::Lexer,
    token::{Token, TokenKind},
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
                "语法错误: 第{}行第{}列: 期望 {:?}，但得到 {:?}",
                self.current.line, self.current.col, expected, self.current.kind
            );
        }
    }

    pub fn parse_stmt(&mut self) -> Statement {
        match &self.current.kind {
            TokenKind::Let => {
                self.advance(); // 吞掉 let

                // 变量名
                let name = match &self.current.kind {
                    TokenKind::Identifier(s) => s.clone(),
                    _ => panic!(
                        "语法错误: 第{}行: let 后面期望变量名，但得到 {:?}",
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
            _ => {
                // 表达式语句：print(y);  add(1, 2);
                let expr = self.parse_expr();
                self.expect(TokenKind::Semicolon);
                self.advance();
                Statement::Expr(Box::new(expr))
            }
        }
    }

    pub fn parse_type(&mut self) -> Type {
        let ty = match self.current.kind {
            TokenKind::I32 => Type::I32,
            TokenKind::Bool => Type::Bool,
            TokenKind::String => Type::String,
            TokenKind::Void => Type::Void,
            _ => panic!(
                "语法错误: 第{}行: 期望类型注解 (i32/bool/string/void)，但得到 {:?}",
                self.current.line, self.current.kind
            ),
        };
        self.advance();
        ty
    }

    // ==================== 表达式解析 ====================

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
        let mut left = self.parse_primary();

        loop {
            let op = match self.current.kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_primary();
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
            TokenKind::IntLiteral(n) => {
                let val = *n;
                self.advance();
                Expression::IntLiteral(val)
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

                // 函数调用：标识符后跟 (
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
            _ => panic!(
                "语法错误: 第{}行: 期望表达式，但得到 {:?}",
                self.current.line, self.current.kind
            ),
        }
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

    // ==================== 函数与程序解析 ====================

    pub fn parse_program(&mut self) -> Program {
        let mut functions = Vec::new();

        while self.current.kind != TokenKind::Eof {
            functions.push(self.parse_function());
        }

        Program { functions }
    }

    pub fn parse_function(&mut self) -> Function {
        self.expect(TokenKind::Fn);
        self.advance(); // 跳过 fn

        // 函数名
        let name = match &self.current.kind {
            TokenKind::Identifier(s) => s.clone(),
            _ => panic!(
                "语法错误: 第{}行: fn 后面期望函数名，但得到 {:?}",
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

        // { 函数体 }
        self.expect(TokenKind::LBrace);
        self.advance();
        let body = self.parse_block();

        Function {
            name,
            params,
            return_type,
            body,
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
                    "语法错误: 第{}行: 期望参数名，但得到 {:?}",
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
