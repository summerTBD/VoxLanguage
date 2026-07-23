use std::path::Path;
use std::process::Command;

use vox_language::{
    vox_codegen::Codegen, vox_lexer::Lexer, vox_parser::Parser, vox_typeck::TypeChecker,
};

fn main() {
    // 读取命令行参数
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: vox <文件.vox>");
        return;
    }

    let input_path = Path::new(&args[1]);
    let source = std::fs::read_to_string(input_path).expect("无法读取源文件");

    // 推导输出文件名：example.vox → example.exe
    let out_name = input_path.with_extension("exe");

    // 1. 词法分析 → 语法分析
    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();
    println!("=== Vox -> AST OK ===");

    // 2. 类型检查
    let mut typeck = TypeChecker::new();
    typeck.check(&program);
    println!("=== Type check OK ===");

    // 3. AST → C 代码
    let codegen = Codegen::new();
    let c_code = codegen.compile(&program);
    println!("\n=== Generated C code ===\n{}", c_code);

    // 3. 写入 C 文件
    let c_path = input_path.with_extension("c");
    std::fs::write(&c_path, &c_code).expect("写入 C 文件失败");

    // 4. gcc 编译（链接 Boehm GC）
    println!("=== gcc compile ===");
    let status = Command::new("gcc")
        .args(&[
            c_path.to_str().unwrap(),
            "-o",
            out_name.to_str().unwrap(),
            "-I",
            r"E:\Dev_Evens\_BoehmGC\gc-8.2.12\include",
            r"E:\Dev_Evens\_BoehmGC\gc-8.2.12\libgc.a",
        ])
        .status()
        .expect("调用 gcc 失败");

    if !status.success() {
        eprintln!("compile failed!");
        return;
    }
    println!("compile OK -> {}", out_name.display());

    // 5. 运行
    println!("\n=== Run {} ===", out_name.display());
    let run = Command::new(format!(".\\{}", out_name.display()))
        .output()
        .expect("运行失败");
    print!("{}", String::from_utf8_lossy(&run.stdout));
}
