use std::process::Command;

use vox_language::{codegen::Codegen, lexer::Lexer, parser::Parser};

fn main() {
    let source = r#"
fn add(a: i32, b: i32): i32 {
    return a + b;
}

fn main() {
    let x: i32 = 10;
    let y: i32 = add(x, 5);
    print(y);
}
"#;

    // 1. 词法分析 → 语法分析
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();
    println!("=== Vox → AST 完成 ===");

    // 2. AST → C 代码
    let codegen = Codegen::new();
    let c_code = codegen.compile(&program);
    println!("\n=== 生成的 C 代码 ===\n{}", c_code);

    // 3. 写入 output.c
    std::fs::write("output.c", &c_code).expect("写入 C 文件失败");

    // 4. gcc 编译 output.c → output.exe
    println!("=== 用 gcc 编译 ===");
    let status = Command::new("gcc")
        .args(&["output.c", "-o", "output.exe"])
        .status()
        .expect("调用 gcc 失败");

    if !status.success() {
        eprintln!("C 编译失败！");
        return;
    }
    println!("编译成功 → output.exe");

    // 5. 运行
    println!("\n=== 运行 output.exe ===");
    let run = Command::new(".\\output.exe").output().expect("运行失败");
    print!("{}", String::from_utf8_lossy(&run.stdout));
}
