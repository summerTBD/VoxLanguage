// Vox 类型检查模块 - v0.2

use std::collections::HashMap;

use crate::vox_ast::{BinOp, Expression, Function, Program, Signedness, Statement, Type};

pub struct TypeChecker {
    /// 函数名 → 返回类型（全局，不清）
    functions: HashMap<String, Type>,
    /// 变量名 → 类型（每函数重新填充）
    variables: HashMap<String, Type>,
    /// 结构体名 → (字段名 → 字段类型)
    structs: HashMap<String, HashMap<String, Type>>,
    // 枚举体名
    enums: HashMap<String, HashMap<String, i32>>,
    /// break/continue 合法性检查
    loop_depth: usize,
    /// 全局 const/static
    globals: HashMap<String, Type>,
    /// #define 宏名 → 类型（类型检查时直接通过）
    defines: HashMap<String, Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            functions: HashMap::new(),
            variables: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            loop_depth: 0,
            globals: HashMap::new(),
            defines: HashMap::new(),
        }
    }

    /// 注册 C 宏名（#define NAME value），类型从 value 推断
    pub fn register_defines(&mut self, names: &[(String, String)]) {
        for (name, value) in names {
            let ty = if value.starts_with('"') {
                Type::Str
            } else if value.contains('.') {
                Type::Float(64)
            } else if value == "true" || value == "false" {
                Type::Bool
            } else {
                Type::Int(Signedness::Signed, 32)
            };
            self.defines.insert(name.clone(), ty);
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

        // 第三遍：注册所有枚举变体
        for e in &program.enums {
            let mut variants = HashMap::new();
            for v in &e.variants {
                variants.insert(v.name.clone(), v.discriminant);
            }
            self.enums.insert(e.name.clone(), variants);
        }

        // 第四遍：验证 const/static 并注册为全局
        for c in &program.consts {
            let actual = self.infer_expr(&c.value);
            if actual != c.type_annot
                && !(c.type_annot.is_integer() && actual.is_integer())
                && !(c.type_annot.is_float() && actual.is_float())
            {
                panic!(
                    "Type error: const '{}' declared {:?} but init is {:?}",
                    c.name, c.type_annot, actual
                );
            }
            self.globals.insert(c.name.clone(), c.type_annot.clone());
        }
        for s in &program.statics {
            let actual = self.infer_expr(&s.value);
            if actual != s.type_annot
                && !(s.type_annot.is_integer() && actual.is_integer())
                && !(s.type_annot.is_float() && actual.is_float())
            {
                panic!(
                    "Type error: static '{}' declared {:?} but init is {:?}",
                    s.name, s.type_annot, actual
                );
            }
            self.globals.insert(s.name.clone(), s.type_annot.clone());
        }

        // 第五遍：检查每个函数体（跳过 extern）
        for func in &program.functions {
            if !func.is_extern {
                self.check_function(func);
            }
        }
    }

    // ==================== 函数 ====================

    fn check_function(&mut self, func: &Function) {
        self.variables.clear();

        // 全局 const/static 对所有函数可见
        for (name, ty) in &self.globals {
            self.variables.insert(name.clone(), ty.clone());
        }

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
                // 字面量允许隐式同族转换（程序员亲手写的值，自己负责）
                let is_literal = matches!(
                    value.as_ref(),
                    Expression::IntLiteral(_) | Expression::FloatLiteral(_)
                );
                let ok = actual == *type_annot
                    || (is_literal && type_annot.is_integer() && actual.is_integer())
                    || (is_literal && type_annot.is_float() && actual.is_float());
                if !ok {
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
                self.loop_depth += 1;
                for stmt in &body.content {
                    self.check_stmt(stmt);
                }
                self.loop_depth -= 1;
            }
            Statement::For {
                init,
                condition,
                step,
                body,
            } => {
                if let Some(init) = init {
                    self.check_stmt(init);
                }
                if let Some(cond) = condition {
                    let ct = self.infer_expr(cond);
                    if ct != Type::Bool {
                        panic!("Type error: for condition must be bool, got {:?}", ct);
                    }
                }
                if let Some(step) = step {
                    self.check_stmt(step);
                }
                self.loop_depth += 1;
                for stmt in &body.content {
                    self.check_stmt(stmt);
                }
                self.loop_depth -= 1;
            }
            Statement::Break => {
                if self.loop_depth == 0 {
                    panic!("Type error: break outside loop");
                }
            }
            Statement::Continue => {
                if self.loop_depth == 0 {
                    panic!("Type error: continue outside loop");
                }
            }
            Statement::Assign { name, value } => {
                let expected = self
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("Type error: undefined variable '{}'", name));
                let actual = self.infer_expr(value);
                let is_literal = matches!(
                    value.as_ref(),
                    Expression::IntLiteral(_) | Expression::FloatLiteral(_)
                );
                let ok = actual == *expected
                    || (is_literal && expected.is_integer() && actual.is_integer())
                    || (is_literal && expected.is_float() && actual.is_float());
                if !ok {
                    panic!(
                        "Type error: var '{}' is {:?}, cannot assign {:?}",
                        name, expected, actual
                    );
                }
            }
            Statement::Store { ptr, value } => {
                let ptr_ty = self.infer_expr(ptr);
                let val_ty = self.infer_expr(value);
                match ptr_ty {
                    Type::Ptr(inner) => {
                        if val_ty != *inner {
                            panic!("Type error: *p expects {:?}, got {:?}", *inner, val_ty);
                        }
                    }
                    _ => panic!(
                        "Type error: cannot dereference non-pointer type {:?}",
                        ptr_ty
                    ),
                }
            }
            Statement::StoreField { object, value, .. } => {
                let _ = self.infer_expr(object);
                let _ = self.infer_expr(value);
            }
            Statement::StoreIndex {
                array,
                index,
                value,
            } => {
                let _arr_ty = self.infer_expr(array);
                let idx_ty = self.infer_expr(index);
                if !idx_ty.is_integer() {
                    panic!("Type error: array index must be integer, got {:?}", idx_ty);
                }
                let _val_ty = self.infer_expr(value);
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
            Expression::IntLiteral(_) => Type::Int(Signedness::Signed, 32),
            Expression::FloatLiteral(_) => Type::Float(64),
            Expression::StringLiteral(_) => Type::Str,
            Expression::BoolLiteral(_) => Type::Bool,
            Expression::Identifier(name) => self
                .variables
                .get(name)
                .cloned()
                .or_else(|| {
                    // #define 宏名
                    self.defines.get(name).cloned()
                })
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
                    // 指针运算：*T + int → *T,  *T - int → *T
                    BinOp::Add | BinOp::Sub => {
                        if matches!(&lt, Type::Ptr(_)) && rt.is_integer() {
                            return lt;
                        }
                        // 普通算术：必须同类型数值
                        if !lt.is_numeric() || !rt.is_numeric() || lt != rt {
                            panic!(
                                "Type error: arithmetic needs same numeric type, got {:?} and {:?}",
                                lt, rt
                            );
                        }
                        lt
                    }
                    // 乘除只支持数值
                    BinOp::Mul | BinOp::Div => {
                        if !lt.is_numeric() || !rt.is_numeric() || lt != rt {
                            panic!(
                                "Type error: mul/div needs same numeric type, got {:?} and {:?}",
                                lt, rt
                            );
                        }
                        lt
                    }
                    // 比较：两边同类型，或指针跟整数比较（null 检查）
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::Gt
                    | BinOp::LtEq
                    | BinOp::GtEq => {
                        // 指针 == 整数 或 指针 == 指针（null 检查）
                        if (matches!(&lt, Type::Ptr(_)) && rt.is_integer())
                            || (lt.is_integer() && matches!(&rt, Type::Ptr(_)))
                        {
                            return Type::Bool;
                        }
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
            Expression::StructLiteral { name, .. } => Type::Adt {
                name: name.clone(),
                args: vec![],
            },
            Expression::New { name, .. } => Type::Ptr(Box::new(Type::Adt {
                name: name.clone(),
                args: vec![],
            })),
            Expression::FieldAccess { object, field } => {
                let obj_ty = self.infer_expr(object);
                // 如果是 *Struct，自动解引用
                let inner_ty = match &obj_ty {
                    Type::Ptr(inner) => inner.as_ref(),
                    other => other,
                };
                match inner_ty {
                    Type::Adt { name, .. } => {
                        if let Some(fields) = self.structs.get(name) {
                            return fields.get(field).cloned().unwrap_or_else(|| {
                                panic!("Type error: struct '{}' has no field '{}'", name, field)
                            });
                        }
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
            Expression::Cast { expr: _, target } => target.clone(),
            Expression::Sizeof(_) => Type::Int(Signedness::Unsigned, 64),
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
                if !idx_ty.is_integer() {
                    panic!("Type error: array index must be integer, got {:?}", idx_ty);
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
                    "free" => {
                        if args.len() != 1 {
                            panic!("Type error: free needs 1 arg(s)");
                        }
                        let arg_ty = self.infer_expr(&args[0]);
                        if !matches!(arg_ty, Type::Ptr(_)) {
                            panic!("Type error: free arg must be a pointer, got {:?}", arg_ty);
                        }
                        Type::Void
                    }
                    "printf" => {
                        // 变参，只检查第一个参数是 str
                        if args.is_empty() {
                            panic!("Type error: printf needs at least 1 arg");
                        }
                        let arg_ty = self.infer_expr(&args[0]);
                        if arg_ty != Type::Str {
                            panic!("Type error: printf fmt must be str, got {:?}", arg_ty);
                        }
                        // 其余参数不检查（变参透传）
                        Type::Int(Signedness::Signed, 32)
                    }
                    "scanf" => {
                        if args.is_empty() {
                            panic!("Type error: scanf needs at least 1 arg");
                        }
                        let arg_ty = self.infer_expr(&args[0]);
                        if arg_ty != Type::Str {
                            panic!("Type error: scanf fmt must be str, got {:?}", arg_ty);
                        }
                        Type::Int(Signedness::Signed, 32)
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
