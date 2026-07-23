// Vox 类型检查模块 - v0.2

use std::collections::HashMap;

use crate::vox_ast::{BinOp, Expression, Function, Program, Statement, Type};

pub struct TypeChecker {
    /// 函数名 → 返回类型（全局，不清）
    functions: HashMap<String, Type>,
    /// 变量名 → 类型（每函数重新填充）
    variables: HashMap<String, Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            functions: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    /// 检查整个程序
    pub fn check(&mut self, program: &Program) {
        // 第一遍：注册所有函数签名
        for func in &program.functions {
            self.functions
                .insert(func.name.clone(), func.return_type.clone());
        }

        // 第二遍：检查每个函数体
        for func in &program.functions {
            self.check_function(func);
        }
    }

    // ==================== 函数 ====================

    fn check_function(&mut self, func: &Function) {
        self.variables.clear();

        // 参数加入符号表
        for param in &func.params {
            self.variables
                .insert(param.name.clone(), param.type_annot.clone());
        }

        // 检查函数体
        for stmt in &func.body.content {
            self.check_stmt(stmt);
        }
    }

    // ==================== 语句 ====================

    fn check_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let {
                name,
                type_annot,
                value,
                ..
            } => {
                let actual = self.infer_expr(value);
                if actual != *type_annot {
                    panic!(
                        "类型错误: 变量 '{}' 声明为 {:?}，但初始值类型是 {:?}",
                        name, type_annot, actual
                    );
                }
                self.variables.insert(name.clone(), type_annot.clone());
            }
            Statement::Return(expr) => {
                if let Some(e) = expr {
                    self.infer_expr(e);
                }
            }
            Statement::Expr(expr) => {
                self.infer_expr(expr);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond_ty = self.infer_expr(condition);
                if cond_ty != Type::Bool {
                    panic!("类型错误: if 条件必须是 bool，但得到 {:?}", cond_ty);
                }
                for stmt in &then_block.content {
                    self.check_stmt(stmt);
                }
                if let Some(else_blk) = else_block {
                    for stmt in &else_blk.content {
                        self.check_stmt(stmt);
                    }
                }
            }
            Statement::While { condition, body } => {
                let cond_ty = self.infer_expr(condition);
                if cond_ty != Type::Bool {
                    panic!("类型错误: while 条件必须是 bool，但得到 {:?}", cond_ty);
                }
                for stmt in &body.content {
                    self.check_stmt(stmt);
                }
            }
            Statement::Assign { name, value } => {
                let expected = self
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("类型错误: 未定义的变量 '{}'", name));
                let actual = self.infer_expr(value);
                if actual != *expected {
                    panic!(
                        "类型错误: 变量 '{}' 类型为 {:?}，不能赋值为 {:?}",
                        name, expected, actual
                    );
                }
            }
        }
    }

    // ==================== 表达式 ====================

    fn infer_expr(&self, expr: &Expression) -> Type {
        match expr {
            Expression::IntLiteral(_) => Type::I32,
            Expression::StringLiteral(_) => Type::String,
            Expression::BoolLiteral(_) => Type::Bool,
            Expression::Identifier(name) => self
                .variables
                .get(name)
                .cloned()
                .unwrap_or_else(|| panic!("类型错误: 未定义的变量 '{}'", name)),
            Expression::Binary { left, op, right } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);

                match op {
                    // 算术：两边必须 i32，返回 i32
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        if lt != Type::I32 || rt != Type::I32 {
                            panic!("类型错误: 算术运算要求 i32，但得到 {:?} 和 {:?}", lt, rt);
                        }
                        Type::I32
                    }
                    // 比较：两边同类型，返回 bool
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::Gt
                    | BinOp::LtEq
                    | BinOp::GtEq => {
                        if lt != rt {
                            panic!("类型错误: 比较运算两边类型不同：{:?} 和 {:?}", lt, rt);
                        }
                        Type::Bool
                    }
                    // 逻辑：两边必须 bool，返回 bool
                    BinOp::And | BinOp::Or => {
                        if lt != Type::Bool || rt != Type::Bool {
                            panic!("类型错误: 逻辑运算要求 bool，但得到 {:?} 和 {:?}", lt, rt);
                        }
                        Type::Bool
                    }
                }
            }
            Expression::Not(inner) => {
                let ty = self.infer_expr(inner);
                if ty != Type::Bool {
                    panic!("类型错误: ! 运算符要求 bool，但得到 {:?}", ty);
                }
                Type::Bool
            }
            Expression::Call { name, args } => {
                match name.as_str() {
                    "print" => {
                        if args.len() != 1 {
                            panic!("类型错误: print 需要 1 个参数");
                        }
                        let arg_ty = self.infer_expr(&args[0]);
                        if arg_ty != Type::I32 && arg_ty != Type::Bool {
                            panic!(
                                "类型错误: print 参数必须是 i32 或 bool，但得到 {:?}",
                                arg_ty
                            );
                        }
                        Type::Void
                    }
                    "read_i32" => {
                        if !args.is_empty() {
                            panic!("类型错误: read_i32 不需要参数");
                        }
                        Type::I32
                    }
                    _ => {
                        // 用户自定义函数
                        let ret_ty = self
                            .functions
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| panic!("类型错误: 未定义的函数 '{}'", name));
                        for arg in args {
                            self.infer_expr(arg);
                        }
                        ret_ty
                    }
                }
            }
        }
    }
}
