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
                        "Type error: var '{}' declared {:?} but init is {:?}",
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
                    panic!("Type error: if condition must be bool, got {:?}", cond_ty);
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
                    panic!("Type error: while condition must be bool, got {:?}", cond_ty);
                }
                for stmt in &body.content {
                    self.check_stmt(stmt);
                }
            }
            Statement::Assign { name, value } => {
                let expected = self
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("Type error: undefined variable '{}'", name));
                let actual = self.infer_expr(value);
                if actual != *expected {
                    panic!(
                        "Type error: var '{}' is {:?}, cannot assign {:?}",
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
            Expression::FloatLiteral(_) => Type::F64,
            Expression::StringLiteral(_) => Type::Str,
            Expression::BoolLiteral(_) => Type::Bool,
            Expression::Identifier(name) => self
                .variables
                .get(name)
                .cloned()
                .unwrap_or_else(|| panic!("Type error: undefined variable '{}'", name)),
            Expression::Binary { left, op, right } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);

                match op {
                    // 算术：两边必须 i32，返回 i32
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        if (lt != Type::I32 && lt != Type::F64)
                            || (rt != Type::I32 && rt != Type::F64)
                            || lt != rt
                        {
                            panic!("Type error: arithmetic needs same type (i32 or f64), got {:?} and {:?}", lt, rt);
                        }
                        lt
                    }
                    // 比较：两边同类型，返回 bool
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::Gt
                    | BinOp::LtEq
                    | BinOp::GtEq => {
                        if lt != rt {
                            panic!("Type error: comparison types differ: {:?} vs {:?}", lt, rt);
                        }
                        Type::Bool
                    }
                    // 逻辑：两边必须 bool，返回 bool
                    BinOp::And | BinOp::Or => {
                        if lt != Type::Bool || rt != Type::Bool {
                            panic!("Type error: logic op needs bool, got {:?} and {:?}", lt, rt);
                        }
                        Type::Bool
                    }
                }
            }
            Expression::Not(inner) => {
                let ty = self.infer_expr(inner);
                if ty != Type::Bool {
                    panic!("Type error: ! needs bool, got {:?}", ty);
                }
                Type::Bool
            }
            Expression::StructLiteral { name, .. } => Type::Struct(name.clone()),
            Expression::New { name, .. } => Type::Struct(name.clone()),
            Expression::FieldAccess { object, field } => {
                let obj_ty = self.infer_expr(object);
                match obj_ty {
                    Type::Struct(_) => Type::I32, // 简化：字段类型未知，默认 i32
                    _ => panic!(
                        "Type error: {:?} is not a struct，不能访问字段 '{}'",
                        obj_ty, field
                    ),
                }
            }
            Expression::AddrOf(inner) => {
                let inner_ty = self.infer_expr(inner);
                Type::Ptr(Box::new(inner_ty))
            }
            Expression::Deref(inner) => {
                let inner_ty = self.infer_expr(inner);
                match inner_ty {
                    Type::Ptr(pointee) => *pointee,
                    _ => panic!(
                        "Type error: cannot dereference non-pointer type {:?}",
                        inner_ty
                    ),
                }
            }
            Expression::Call { name, args } => {
                match name.as_str() {
                    "print" => {
                        if args.len() != 1 {
                            panic!("Type error: print needs 1 arg(s)");
                        }
                        let arg_ty = self.infer_expr(&args[0]);
                        if arg_ty != Type::I32 && arg_ty != Type::Bool {
                            panic!(
                                "Type error: print arg must be i32 or bool, got {:?}",
                                arg_ty
                            );
                        }
                        Type::Void
                    }
                    "print_str" => {
                        if args.len() != 1 {
                            panic!("Type error: print_str needs 1 arg(s)");
                        }
                        let arg_ty = self.infer_expr(&args[0]);
                        if arg_ty != Type::Str {
                            panic!("Type error: print_str arg must be str, got {:?}", arg_ty);
                        }
                        Type::Void
                    }
                    "print_f64" => {
                        if args.len() != 1 {
                            panic!("Type error: print_f64 needs 1 arg(s)");
                        }
                        let arg_ty = self.infer_expr(&args[0]);
                        if arg_ty != Type::F64 {
                            panic!("Type error: print_f64 arg must be f64, got {:?}", arg_ty);
                        }
                        Type::Void
                    }
                    "read_i32" => {
                        if !args.is_empty() {
                            panic!("Type error: read_i32 takes no args");
                        }
                        Type::I32
                    }
                    _ => {
                        // 用户自定义函数
                        let ret_ty = self
                            .functions
                            .get(name)
                            .cloned()
                            .unwrap_or_else(|| panic!("Type error: undefined function '{}'", name));
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

