use std::path::{Path, PathBuf};
use std::process::Command;

use vox_language::{
    vox_codegen::Codegen, vox_lexer::Lexer, vox_parser::Parser, vox_typeck::TypeChecker,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 解析参数
    let mut no_gc = false;
    let mut check_only = false;
    let mut out_dir: Option<String> = None;
    let mut input_file: Option<&str> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--no-gc" => no_gc = true,
            "--check" => check_only = true,
            "--out" => {
                i += 1;
                if i < args.len() {
                    out_dir = Some(args[i].clone());
                }
            }
            _ => input_file = Some(&args[i]),
        }
        i += 1;
    }

    let input_path = match input_file {
        Some(f) => Path::new(f),
        None => {
            eprintln!("用法: vox [--no-gc] <文件.vox>");
            return;
        }
    };
    let user_source = std::fs::read_to_string(input_path).expect("无法读取源文件");

    // 预处理器：展开所有 mod 指令
    let base_dir = input_path.parent().unwrap_or(Path::new("."));
    let user_source = vox_language::expand_mods(&user_source, base_dir);

    // 预处理器：文本替换 #define 名称（其余 # 由 lexer/parser 处理）
    let (define_names, user_source) = vox_language::replace_defines(&user_source);

    // 拼接 prelude（C 标准函数声明） + 用户源码
    let prelude = include_str!("../prelude.vox");
    let source = format!("{}\n{}", prelude, user_source);

    // 输出目录
    let out_dir = out_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| input_path.parent().unwrap_or(Path::new(".")).to_path_buf());
    std::fs::create_dir_all(&out_dir).expect("无法创建输出目录");

    let stem = input_path.file_stem().unwrap().to_str().unwrap();
    let c_path = out_dir.join(format!("{}.c", stem));
    let exe_path = out_dir.join(format!("{}.exe", stem));

    // 1. 词法分析 → 语法分析
    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();
    println!("=== Vox -> AST OK ===");

    // [DEPRECATED] 单态化，泛型搁置后暂不使用
    // vox_mono::monomorphize(&mut program);
    // println!("=== Monomorphize OK ===");

    // 3. 类型检查
    let mut typeck = TypeChecker::new();
    typeck.register_defines(&define_names);
    typeck.check(&program);

    if check_only {
        std::process::exit(0);
    }

    println!("=== Type check OK ===");

    // 4. AST → C 代码
    let codegen = Codegen::new(!no_gc);
    let c_code = codegen.compile(&program);

    // 写入 C 文件
    std::fs::write(&c_path, &c_code).expect("写入 C 文件失败");

    // gcc 编译
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    let gc_include = exe_dir.join("vendor/include");
    let gc_lib = exe_dir.join("vendor/libgc.a");

    println!("=== gcc compile ===");
    let mut gcc_args = vec![c_path.to_str().unwrap(), "-o", exe_path.to_str().unwrap()];
    if !no_gc {
        gcc_args.push("-I");
        gcc_args.push(gc_include.to_str().unwrap());
        gcc_args.push(gc_lib.to_str().unwrap());
    }
    let status = Command::new("gcc")
        .args(&gcc_args)
        .status()
        .expect("调用 gcc 失败");

    if !status.success() {
        eprintln!("compile failed!");
        return;
    }
    println!("compile OK -> {}", exe_path.display());

    // 运行
    println!("\n=== Run {} ===", exe_path.display());
    let run = Command::new(exe_path.to_str().unwrap())
        .output()
        .expect("运行失败");
    print!("{}", String::from_utf8_lossy(&run.stdout));
}
