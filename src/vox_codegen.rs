// Vox 代码生成模块 - 翻译 AST 到 C 源码
// v0.1 最小子集

use std::collections::HashSet;

use crate::vox_ast::{BinOp, Expression, Function, Program, Signedness, Statement, Type};

pub struct Codegen {
    output: String,
    indent_level: usize,
    tmp_counter: u32,
    enum_names: HashSet<String>,
    ptr_vars: HashSet<String>,
    use_gc: bool,
    cpp_directives: String,
}

impl Codegen {
    pub fn new(use_gc: bool, cpp_directives: String) -> Self {
        Codegen {
            output: String::new(),
            indent_level: 0,
            tmp_counter: 0,
            enum_names: HashSet::new(),
            ptr_vars: HashSet::new(),
            use_gc,
            cpp_directives,
        }
    }

    fn fresh_tmp(&mut self) -> String {
        let n = self.tmp_counter;
        self.tmp_counter += 1;
        format!("_t{}", n)
    }

    fn alloc_fn(&self) -> &str {
        if self.use_gc {
            "GC_malloc"
        } else {
            "malloc"
        }
    }

    fn free_fn(&self) -> &str {
        if self.use_gc {
            "GC_free"
        } else {
            "free"
        }
    }

    /// 编译整个程序，返回 C 源码
    pub fn compile(mut self, program: &Program) -> String {
        // 收集枚举名
        for e in &program.enums {
            self.enum_names.insert(e.name.clone());
        }

        // C 预处理指令（#define / #include）—— 透传到 C 代码最顶部
        if !self.cpp_directives.is_empty() {
            self.emit("// === C 预处理指令 ===");
            let dirs = std::mem::take(&mut self.cpp_directives);
            for line in dirs.lines() {
                self.output.push_str(line);
                self.output.push('\n');
            }
            self.emit("");
        }

        // 头文件
        self.emit("#include <stdint.h>");
        if self.use_gc {
            self.emit("#include <gc.h>");
        }
        self.emit("");
        // 非 GC 模式
        if !self.use_gc {
            self.emit("// === 非 GC 模式 ===");
            self.emit("#include <stdlib.h>");
            self.emit("");
        }
        // 运行时内部依赖的 C 函数（不暴露给 Vox 用户）
        self.emit("// === C 运行时依赖 ===");
        self.emit("extern int printf(const char* fmt, ...);");
        self.emit("extern int scanf(const char* fmt, ...);");
        self.emit("extern int puts(const char* s);");
        self.emit("");

        // struct 定义
        if !program.structs.is_empty() {
            self.emit("// === 结构体定义 ===");
            for s in &program.structs {
                self.compile_struct_def(s);
            }
            self.emit("");
        }

        // enum 定义
        if !program.enums.is_empty() {
            self.emit("// === 枚举定义 ===");
            for e in &program.enums {
                self.compile_enum_def(e);
            }
            self.emit("");
        }

        // const 定义
        for c in &program.consts {
            let ty = self.type_to_c(&c.type_annot);
            let val = self.compile_expr(&c.value);
            self.emit(&format!("static const {} {} = {};", ty, c.name, val));
        }
        if !program.consts.is_empty() {
            self.emit("");
        }

        // static 定义
        for s in &program.statics {
            let ty = self.type_to_c(&s.type_annot);
            let val = self.compile_expr(&s.value);
            self.emit(&format!("static {} {} = {};", ty, s.name, val));
        }
        if !program.statics.is_empty() {
            self.emit("");
        }

        // 函数声明（去重：prelude 中的 extern 优先）
        self.emit("// === 函数声明 ===");
        let mut declared = std::collections::HashSet::new();
        for func in &program.functions {
            if !declared.contains(&func.name) {
                self.emit_function_decl(func);
                declared.insert(func.name.clone());
            }
        }
        self.emit("");

        // 函数定义（跳过 extern）
        self.emit("// === 函数定义 ===");
        for func in &program.functions {
            if !func.is_extern {
                self.compile_function(func);
            }
        }

        self.output
    }

    // ==================== 工具方法 ====================

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    fn emit(&mut self, line: &str) {
        let pad = "    ".repeat(self.indent_level);
        self.output.push_str(&format!("{}{}\n", pad, line));
    }

    fn type_to_c(&self, ty: &Type) -> String {
        match ty {
            Type::Int(sign, bits) => match (sign, bits) {
                (Signedness::Signed, 8) => "int8_t",
                (Signedness::Signed, 16) => "int16_t",
                (Signedness::Signed, 32) => "int32_t",
                (Signedness::Signed, 64) => "int64_t",
                (Signedness::Unsigned, 8) => "uint8_t",
                (Signedness::Unsigned, 16) => "uint16_t",
                (Signedness::Unsigned, 32) => "uint32_t",
                (Signedness::Unsigned, 64) => "uint64_t",
                _ => panic!("Unsupported integer type: {:?}", ty),
            }
            .to_string(),
            Type::Float(bits) => match bits {
                32 => "float",
                64 => "double",
                _ => panic!("Unsupported float type: {:?}", ty),
            }
            .to_string(),
            Type::Bool => "int".to_string(),
            Type::Char => "char".to_string(),
            Type::Str => "const char*".to_string(),
            Type::Void => "void".to_string(),
            Type::Adt { name, .. } => {
                if self.enum_names.contains(name) {
                    format!("enum {}", name)
                } else {
                    format!("struct {}", name)
                }
            }
            Type::Ptr(inner) => format!("{}*", self.type_to_c(inner)),
            Type::Array(elem, size) => format!("{}[{}]", self.type_to_c(elem), size),
        }
    }

    /// C 函数的返回类型。main 强制为 int（C 标准要求）
    /// struct 返回值自动加 *（Vox 中 struct 都是堆指针）
    fn ret_type_to_c(&self, func: &Function) -> String {
        if func.name == "main" {
            "int".to_string()
        } else {
            self.type_to_c(&func.return_type)
        }
    }

    fn compile_struct_def(&mut self, s: &crate::vox_ast::StructDef) {
        self.emit(&format!("struct {} {{", s.name));
        self.indent();
        for field in &s.fields {
            let ty = self.type_to_c(&field.type_annot);
            self.emit(&format!("{} {};", ty, field.name));
        }
        self.dedent();
        self.emit("};");
    }

    fn compile_enum_def(&mut self, e: &crate::vox_ast::EnumDef) {
        self.emit(&format!("enum {} {{", e.name));
        self.indent();
        for v in &e.variants {
            self.emit(&format!("{} = {},", v.name, v.discriminant));
        }
        self.dedent();
        self.emit("};");
    }

    // ==================== 函数 ====================

    fn emit_function_decl(&mut self, func: &Function) {
        let ret = self.ret_type_to_c(func);
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{} {}", self.type_to_c(&p.type_annot), p.name))
            .collect();
        self.emit(&format!("{} {}({});", ret, func.name, params.join(", ")));
    }

    fn compile_function(&mut self, func: &Function) {
        // 指针参数加入 ptr_vars
        for p in &func.params {
            if matches!(p.type_annot, Type::Ptr(_)) {
                self.ptr_vars.insert(p.name.clone());
            }
        }

        let ret = self.ret_type_to_c(func);
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{} {}", self.type_to_c(&p.type_annot), p.name))
            .collect();

        self.emit(&format!("{} {}({}) {{", ret, func.name, params.join(", ")));
        self.indent();

        for stmt in &func.body.content {
            self.compile_stmt(stmt);
        }

        // 无返回值的函数隐式 return；main 强制 return 0
        if func.name == "main" {
            self.emit("return 0;");
        } else if func.return_type == Type::Void {
            self.emit("return;");
        }

        self.dedent();
        self.emit("}");
        self.emit("");
    }

    // ==================== 语句 ====================

    fn compile_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let {
                name,
                type_annot,
                value,
                mutable,
            } => {
                // 数组声明特殊处理：let arr: [i32; 3] = [1, 2, 3];
                if let Type::Array(elem_ty, _size) = type_annot {
                    if let Expression::ArrayLiteral(elements) = value.as_ref() {
                        let elem_c = self.type_to_c(elem_ty);
                        let inits: Vec<String> =
                            elements.iter().map(|e| self.compile_expr(e)).collect();
                        self.emit(&format!(
                            "{} {}[{}] = {{ {} }};",
                            elem_c,
                            name,
                            elements.len(),
                            inits.join(", ")
                        ));
                        return;
                    }
                }

                // 堆分配 (new X{...}) 特殊处理
                if let Expression::New {
                    name: struct_name,
                    fields,
                } = value.as_ref()
                {
                    self.ptr_vars.insert(name.clone()); // new → 指针
                    self.emit(&format!(
                        "struct {}* {} = {}(sizeof(struct {}));",
                        struct_name,
                        name,
                        self.alloc_fn(),
                        struct_name
                    ));
                    for (f, v) in fields {
                        let val = self.compile_expr(v);
                        self.emit(&format!("{}->{} = {};", name, f, val));
                    }
                } else {
                    // 指针声明显式追踪
                    if matches!(type_annot, Type::Ptr(_)) {
                        self.ptr_vars.insert(name.clone());
                    }
                    let ty = self.type_to_c(type_annot);
                    let val = self.compile_expr(value);
                    if *mutable {
                        self.emit(&format!("{} {} = {};", ty, name, val));
                    } else {
                        self.emit(&format!("{} const {} = {};", ty, name, val));
                    }
                }
            }
            Statement::Return(expr) => {
                if let Some(e) = expr {
                    let val = self.compile_expr(e);
                    self.emit(&format!("return {};", val));
                } else {
                    self.emit("return;");
                }
            }
            Statement::Expr(expr) => {
                let val = self.compile_expr(expr);
                self.emit(&format!("{};", val));
            }
            Statement::Assign { name, value } => {
                let val = self.compile_expr(value);
                self.emit(&format!("{} = {};", name, val));
            }
            Statement::Store { ptr, value } => {
                let p = self.compile_expr(ptr);
                let v = self.compile_expr(value);
                self.emit(&format!("*{} = {};", p, v));
            }
            Statement::StoreField {
                object,
                field,
                value,
            } => {
                let obj = self.compile_expr(object);
                let v = self.compile_expr(value);
                if matches!(object.as_ref(), Expression::Identifier(_)) {
                    self.emit(&format!("{}->{} = {};", obj, field, v));
                } else {
                    self.emit(&format!("{}.{} = {};", obj, field, v));
                }
            }
            Statement::StoreIndex {
                array,
                index,
                value,
            } => {
                let arr = self.compile_expr(array);
                let idx = self.compile_expr(index);
                let v = self.compile_expr(value);
                self.emit(&format!("{}[{}] = {};", arr, idx, v));
            }
            Statement::Match { expr, arms } => {
                let val = self.compile_expr(expr);
                self.emit(&format!("switch ({}) {{", val));
                self.indent();
                for arm in arms {
                    self.emit(&format!("case {}: {{", arm.pattern));
                    self.indent();
                    for stmt in &arm.body.content {
                        self.compile_stmt(stmt);
                    }
                    self.emit("break;");
                    self.dedent();
                    self.emit("}");
                }
                self.dedent();
                self.emit("}");
            }
            Statement::While { condition, body } => {
                let cond = self.compile_expr(condition);
                self.emit(&format!("while ({}) {{", cond));
                self.indent();
                for stmt in &body.content {
                    self.compile_stmt(stmt);
                }
                self.dedent();
                self.emit("}");
            }
            Statement::For {
                init,
                condition,
                step,
                body,
            } => {
                let init_str = match init.as_ref() {
                    Some(stmt) => {
                        // Let 语句 → "int32_t i = 0"（去掉末尾分号）
                        let prev = std::mem::take(&mut self.output);
                        self.output = String::new();
                        self.compile_stmt(stmt);
                        let line = std::mem::take(&mut self.output);
                        self.output = prev;
                        line.trim().trim_end_matches(';').to_string()
                    }
                    None => String::new(),
                };
                let cond_str = condition
                    .as_ref()
                    .map(|e| self.compile_expr(e))
                    .unwrap_or_default();
                let step_str = match step.as_ref() {
                    Some(stmt) => {
                        let prev = std::mem::take(&mut self.output);
                        self.output = String::new();
                        self.compile_stmt(stmt);
                        let line = std::mem::take(&mut self.output);
                        self.output = prev;
                        line.trim().trim_end_matches(';').to_string()
                    }
                    None => String::new(),
                };
                self.emit(&format!(
                    "for ({}; {}; {}) {{",
                    init_str, cond_str, step_str
                ));
                self.indent();
                for stmt in &body.content {
                    self.compile_stmt(stmt);
                }
                self.dedent();
                self.emit("}");
            }
            Statement::Break => {
                self.emit("break;");
            }
            Statement::Continue => {
                self.emit("continue;");
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond = self.compile_expr(condition);
                self.emit(&format!("if ({}) {{", cond));
                self.indent();
                for stmt in &then_block.content {
                    self.compile_stmt(stmt);
                }
                self.dedent();

                if let Some(else_blk) = else_block {
                    self.emit("} else {");
                    self.indent();
                    for stmt in &else_blk.content {
                        self.compile_stmt(stmt);
                    }
                    self.dedent();
                }
                self.emit("}");
            }
        }
    }

    // ==================== 表达式 ====================

    fn compile_expr(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::IntLiteral(n) => n.to_string(),
            Expression::FloatLiteral(n) => n.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s),
            Expression::BoolLiteral(b) => {
                if *b {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            Expression::Identifier(name) => name.clone(),
            Expression::Binary { left, op, right } => {
                let l = self.compile_expr(left);
                let r = self.compile_expr(right);
                let op_str = self.binop_to_c(op);
                format!("({} {} {})", l, op_str, r)
            }
            Expression::Not(inner) => {
                let val = self.compile_expr(inner);
                format!("!{val}")
            }
            Expression::Call { name, args } => {
                if name == "free" {
                    let arg = self.compile_expr(&args[0]);
                    return format!("{}({})", self.free_fn(), arg);
                }
                let args_str: Vec<String> = args.iter().map(|a| self.compile_expr(a)).collect();
                format!("{}({})", name, args_str.join(", "))
            }
            Expression::StructLiteral { name, fields } => {
                let inits: Vec<String> = fields
                    .iter()
                    .map(|(f, v)| format!(".{} = {}", f, self.compile_expr(v)))
                    .collect();
                format!("(struct {}){{ {} }}", name, inits.join(", "))
            }
            Expression::New { name, fields } => {
                let tmp = self.fresh_tmp();
                self.emit(&format!(
                    "struct {}* {} = {}(sizeof(struct {}));",
                    name,
                    tmp,
                    self.alloc_fn(),
                    name
                ));
                for (f, v) in fields {
                    let val = self.compile_expr(v);
                    self.emit(&format!("{}->{} = {};", tmp, f, val));
                }
                tmp
            }
            Expression::FieldAccess { object, field } => {
                let obj = self.compile_expr(object);
                // 枚举访问：Color.Red → 直接输出 Red
                if let Expression::Identifier(obj_name) = object.as_ref() {
                    if self.enum_names.contains(obj_name) {
                        return field.clone();
                    }
                    if self.ptr_vars.contains(obj_name) {
                        return format!("{}->{}", obj, field);
                    }
                    return format!("{}.{}", obj, field);
                }
                // 链式访问（sb->inner.len）：非标识符对象大概率是指针，用 ->
                let use_arrow = matches!(
                    object.as_ref(),
                    Expression::FieldAccess { .. }
                        | Expression::Deref(..)
                        | Expression::Index { .. }
                );
                if use_arrow {
                    format!("{}->{}", obj, field)
                } else {
                    format!("{}.{}", obj, field)
                }
            }
            Expression::AddrOf(inner) => {
                let val = self.compile_expr(inner);
                format!("(&{})", val)
            }
            Expression::Cast { expr, target } => {
                let val = self.compile_expr(expr);
                let ty = self.type_to_c(target);
                format!("(({}){})", ty, val)
            }
            Expression::Sizeof(ty) => {
                format!("sizeof({})", self.type_to_c(ty))
            }
            Expression::Deref(inner) => {
                let val = self.compile_expr(inner);
                format!("(*{})", val)
            }
            Expression::ArrayLiteral(elements) => {
                let inits: Vec<String> = elements.iter().map(|e| self.compile_expr(e)).collect();
                format!("{{ {} }}", inits.join(", "))
            }
            Expression::Index { array, index } => {
                let arr = self.compile_expr(array);
                let idx = self.compile_expr(index);
                format!("{}[{}]", arr, idx)
            }
        }
    }

    fn binop_to_c(&self, op: &BinOp) -> &str {
        match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Eq => "==",
            BinOp::NotEq => "!=",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::LtEq => "<=",
            BinOp::GtEq => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }
}
