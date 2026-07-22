// Vox 代码生成模块 - 翻译 AST 到 C 源码
// v0.1 最小子集

use crate::ast::{BinOp, Expression, Function, Program, Statement, Type};

pub struct Codegen {
    output: String,
    indent_level: usize,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            output: String::new(),
            indent_level: 0,
        }
    }

    /// 编译整个程序，返回 C 源码
    pub fn compile(mut self, program: &Program) -> String {
        // 头文件
        self.emit("#include <stdint.h>");
        self.emit("#include <stdio.h>");
        self.emit("");
        self.emit("// === Vox 运行时 ===");
        self.emit("static int32_t print(int32_t x) {");
        self.indent();
        self.emit("printf(\"%d\\n\", x);");
        self.emit("return 0;");
        self.dedent();
        self.emit("}");
        self.emit("");

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

    fn type_to_c(&self, ty: &Type) -> &str {
        match ty {
            Type::I32 => "int32_t",
            Type::Bool => "int",
            Type::String => "char*",
            Type::Void => "void",
        }
    }

    /// C 函数的返回类型。main 强制为 int（C 标准要求）
    fn ret_type_to_c(&self, func: &Function) -> &str {
        if func.name == "main" {
            "int"
        } else {
            self.type_to_c(&func.return_type)
        }
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
                ..
            } => {
                let ty = self.type_to_c(type_annot);
                let val = self.compile_expr(value);

                if *mutable {
                    self.emit(&format!("{} {} = {};", ty, name, val));
                } else {
                    self.emit(&format!("{} const {} = {};", ty, name, val));
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

    fn compile_expr(&self, expr: &Expression) -> String {
        match expr {
            Expression::IntLiteral(n) => n.to_string(),
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
            Expression::Call { name, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.compile_expr(a)).collect();
                format!("{}({})", name, args_str.join(", "))
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
