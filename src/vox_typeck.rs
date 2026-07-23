// Vox 类型检查模块 - v0.2

use std::collections::HashMap;

use crate::vox_ast::{BinOp, Expression, Function, Program, Statement, Type};

pub struct TypeChecker {
    /// 函数名 → 返回类型（全局，不清）
    functions: HashMap<String, Type>,
    /// 变量名 → 类型（每函数重新填充）
    variables: HashMap<String, Type>,
    /// 结构体名 → (字段名 → 字段类型)
    structs: HashMap<String, HashMap<String, Type>>,
    // 枚举体名
    enums: HashMap<String, HashMap<String, i32>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            functions: HashMap::new(),
            variables: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
        }
    }

    /// 检查整个程序
    pub fn check(&mut self, program: &Program) {
        // 第一遍：注册所有结构体字段
        for s in &program.structs {
            let mut fields = HashMap::new();
            for f in &s.fields {
                fields.insert(f.name.clone(), f.type_annot.clone());
            }
            self.structs.insert(s.name.clone(), fields);
        }

        // 第二遍：注册所有函数签名
        for func in &program.functions {
            self.functions
                .insert(func.name.clone(), func.return_type.clone());
        }

        // 第三遍：注册所有枚举变体
        for e in &program.enums {
            let mut variants = HashMap::new();
            for v in &e.variants {
                variants.insert(v.name.clone(), v.discriminant);
            }
            self.enums.insert(e.name.clone(), variants);
        }

        // 第四遍：检查每个函数体
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
                    panic!(
                        "Type error: while condition must be bool, got {:?}",
                        cond_ty
                    );
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
            Statement::Match { expr, arms } => {
                let ty = self.infer_expr(expr);
                match &ty {
                    Type::Adt { name, .. } => {
                        let variants = self
                            .enums
                            .get(name)
                            .unwrap_or_else(|| panic!("Type error: undefined enum '{}'", name));
                        // 先验证所有 pattern 有效
                        for arm in arms.iter() {
                            if !variants.contains_key(&arm.pattern) {
                                panic!(
                                    "Type error: enum '{}' has no variant '{}'",
                                    name, arm.pattern
                                );
                            }
                        }
                    }
                    _ => panic!("Type error: match requires enum, got {:?}", ty),
                }
                // 再检查各 arm 的 body（需要 mut borrow）
                for arm in arms {
                    for stmt in &arm.body.content {
                        self.check_stmt(stmt);
                    }
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
                .or_else(|| {
                    // 不是变量，检查是否是类型名（struct 或 enum）
                    if self.enums.contains_key(name) || self.structs.contains_key(name) {
                        Some(Type::Adt {
                            name: name.clone(),
                            args: vec![],
                        })
                    } else {
                        None
                    }
                })
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
            Expression::New { name, .. } => Type::Adt {
                name: name.clone(),
                args: vec![],
            },
            Expression::FieldAccess { object, field } => {
                let obj_ty = self.infer_expr(object);
                match &obj_ty {
                    Type::Adt { name, .. } => {
                        // 先查 struct 字段
                        if let Some(fields) = self.structs.get(name) {
                            return fields.get(field).cloned().unwrap_or_else(|| {
                                panic!("Type error: struct '{}' has no field '{}'", name, field)
                            });
                        }
                        // 再查 enum 变体
                        if let Some(variants) = self.enums.get(name) {
                            if !variants.contains_key(field) {
                                panic!("Type error: enum '{}' has no variant '{}'", name, field);
                            }
                            return Type::Adt {
                                name: name.clone(),
                                args: vec![],
                            };
                        }
                        panic!("Type error: undefined type '{}'", name);
                    }
                    _ => panic!(
                        "Type error: {:?} is not a struct or enum，不能访问成员 '{}'",
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
            Expression::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    panic!("Type error: empty array literal needs type annotation");
                }
                let elem_ty = self.infer_expr(&elements[0]);
                for (i, elem) in elements.iter().enumerate().skip(1) {
                    let ty = self.infer_expr(elem);
                    if ty != elem_ty {
                        panic!(
                            "Type error: array element {} type {:?} doesn't match {:?}",
                            i, ty, elem_ty
                        );
                    }
                }
                Type::Array(Box::new(elem_ty), elements.len())
            }
            Expression::Index { array, index } => {
                let arr_ty = self.infer_expr(array);
                let idx_ty = self.infer_expr(index);
                if idx_ty != Type::I32 {
                    panic!("Type error: array index must be i32, got {:?}", idx_ty);
                }
                match arr_ty {
                    Type::Array(elem_ty, _) => *elem_ty,
                    Type::Ptr(elem_ty) => *elem_ty,
                    _ => panic!(
                        "Type error: cannot index non-array/non-pointer type {:?}",
                        arr_ty
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
                        if arg_ty != Type::I32
                            && arg_ty != Type::Bool
                            && !matches!(arg_ty, Type::Adt { .. })
                        {
                            panic!(
                                "Type error: print arg must be i32, bool or enum, got {:?}",
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
                        let ret_ty =
                            self.functions.get(name).cloned().unwrap_or_else(|| {
                                panic!("Type error: undefined function '{}'", name)
                            });
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
