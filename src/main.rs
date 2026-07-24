use std::path::Path;
use std::process::Command;

use vox_language::{
    vox_codegen::Codegen, vox_lexer::Lexer, vox_parser::Parser, vox_typeck::TypeChecker,
};

/// 预处理器：提取 C 预处理指令，同时收集 #define 名称供 typeck 识别
fn extract_cpp_directives(source: &str) -> (String, Vec<(String, String)>) {
    let mut directives = String::new();
    let mut define_names = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            directives.push_str(line);
            directives.push('\n');
            if let Some(rest) = trimmed.strip_prefix("#define ") {
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let name = parts[0].to_string();
                    let value = parts[1].trim().to_string();
                    define_names.push((name, value));
                }
            }
        }
    }
    (directives, define_names)
}

/// 预处理器：展开 mod "file.vox"; 指令（支持嵌套）
fn expand_mods(source: &str, base_dir: &Path) -> String {
    let mut result = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("mod \"") && trimmed.ends_with("\";") {
            let path = &trimmed[5..trimmed.len() - 2];
            let mod_path = base_dir.join(path);
            let mod_src = std::fs::read_to_string(&mod_path)
                .unwrap_or_else(|_| panic!("无法读取模块: {}", mod_path.display()));
            let mod_dir = mod_path.parent().unwrap_or(base_dir);
            result.push_str(&expand_mods(&mod_src, mod_dir));
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // 解析参数
    let mut no_gc = false;
    let mut input_file: Option<&str> = None;
    for arg in &args[1..] {
        if arg == "--no-gc" {
            no_gc = true;
        } else {
            input_file = Some(arg);
        }
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
    let user_source = expand_mods(&user_source, base_dir);

    // 预处理器：提取 C 宏（#define / #include）→ 透传到 C 代码顶部
    let (cpp_directives, define_names) = extract_cpp_directives(&user_source);
    // 移除 # 行
    let user_source: String = user_source
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    // 拼接 prelude（C 标准函数声明） + 用户源码
    let prelude = include_str!("../prelude.vox");
    let source = format!("{}\n{}", prelude, user_source);

    // 推导输出文件名：example.vox → example.exe
    let out_name = input_path.with_extension("exe");

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
    println!("=== Type check OK ===");

    // 4. AST → C 代码
    let codegen = Codegen::new(!no_gc, cpp_directives);
    let c_code = codegen.compile(&program);
    println!("\n=== Generated C code ===\n{}", c_code);

    // 3. 写入 C 文件
    let c_path = input_path.with_extension("c");
    std::fs::write(&c_path, &c_code).expect("写入 C 文件失败");

    // 4. gcc 编译（链接 Boehm GC，路径相对于 voxc.exe）
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    let gc_include = exe_dir.join("../vendor/gc/include");
    let gc_lib = exe_dir.join("../vendor/gc/libgc.a");

    println!("=== gcc compile ===");
    let mut gcc_args = vec![c_path.to_str().unwrap(), "-o", out_name.to_str().unwrap()];
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
    println!("compile OK -> {}", out_name.display());

    // 5. 运行
    println!("\n=== Run {} ===", out_name.display());
    let run = Command::new(format!(".\\{}", out_name.display()))
        .output()
        .expect("运行失败");
    print!("{}", String::from_utf8_lossy(&run.stdout));
}
