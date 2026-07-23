// Vox 代码生成模块 - 翻译 AST 到 C 源码
// v0.1 最小子集

use std::collections::HashSet;

use crate::vox_ast::{BinOp, Expression, Function, Program, Statement, Type};

pub struct Codegen {
    output: String,
    indent_level: usize,
    tmp_counter: u32,
    enum_names: HashSet<String>,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            output: String::new(),
            indent_level: 0,
            tmp_counter: 0,
            enum_names: HashSet::new(),
        }
    }

    fn fresh_tmp(&mut self) -> String {
        let n = self.tmp_counter;
        self.tmp_counter += 1;
        format!("_t{}", n)
    }

    /// 编译整个程序，返回 C 源码
    pub fn compile(mut self, program: &Program) -> String {
        // 收集枚举名
        for e in &program.enums {
            self.enum_names.insert(e.name.clone());
        }

        // 头文件
        self.emit("#include <stdint.h>");
        self.emit("#include <stdio.h>");
        self.emit("#include <gc.h>");
        self.emit("");
        self.emit("// === Vox 运行时 ===");
        self.emit("static int32_t print(int32_t x) {");
        self.indent();
        self.emit("printf(\"%d\\n\", x);");
        self.emit("return 0;");
        self.dedent();
        self.emit("}");
        self.emit("");
        self.emit("static int32_t read_i32() {");
        self.indent();
        self.emit("int32_t x;");
        self.emit("scanf(\"%d\", &x);");
        self.emit("return x;");
        self.dedent();
        self.emit("}");
        self.emit("");
        self.emit("static void print_f64(double x) {");
        self.indent();
        self.emit("printf(\"%f\\n\", x);");
        self.dedent();
        self.emit("}");
        self.emit("");
        self.emit("static void print_str(const char* s) {");
        self.indent();
        self.emit("printf(\"%s\\n\", s);");
        self.dedent();
        self.emit("}");
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

        // 函数声明 —— 先声明所有函数，支持互相调用
        self.emit("// === 函数声明 ===");
        for func in &program.functions {
            self.emit_function_decl(func);
        }
        self.emit("");

        // 函数定义
        self.emit("// === 函数定义 ===");
        for func in &program.functions {
            self.compile_function(func);
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
            Type::I32 => "int32_t".to_string(),
            Type::Bool => "int".to_string(),
            Type::Str => "const char*".to_string(),
            Type::F64 => "double".to_string(),
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
                    self.emit(&format!(
                        "struct {}* {} = GC_malloc(sizeof(struct {}));",
                        struct_name, name, struct_name
                    ));
                    for (f, v) in fields {
                        let val = self.compile_expr(v);
                        self.emit(&format!("{}->{} = {};", name, f, val));
                    }
                } else {
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
                let args_str: Vec<String> = args.iter().map(|a| self.compile_expr(a)).collect();
                format!("{}({})", name, args_str.join(", "))
            }
            Expression::New { name, fields } => {
                let tmp = self.fresh_tmp();
                self.emit(&format!(
                    "struct {}* {} = GC_malloc(sizeof(struct {}));",
                    name, tmp, name
                ));
                for (f, v) in fields {
                    let val = self.compile_expr(v);
                    self.emit(&format!("{}->{} = {};", tmp, f, val));
                }
                tmp
            }
            Expression::FieldAccess { object, field } => {
                // 枚举访问：Color.Red → 直接输出 Red
                if let Expression::Identifier(obj_name) = object.as_ref() {
                    if self.enum_names.contains(obj_name) {
                        return field.clone();
                    }
                }
                let obj = self.compile_expr(object);
                if matches!(object.as_ref(), Expression::Identifier(_)) {
                    format!("{}->{}", obj, field)
                } else {
                    format!("{}.{}", obj, field)
                }
            }
            Expression::AddrOf(inner) => {
                let val = self.compile_expr(inner);
                format!("(&{})", val)
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
