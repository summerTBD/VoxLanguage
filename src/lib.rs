pub mod vox_ast;
pub mod vox_codegen;
pub mod vox_lexer;
pub mod vox_parser;
pub mod vox_token;
pub mod vox_typeck;

use std::collections::HashSet;
use std::path::Path;

/// 预处理器：展开 mod "file.vox"; 指令（支持嵌套，自动去重）
pub fn expand_mods(source: &str, base_dir: &Path) -> String {
    expand_mods_impl(source, base_dir, &mut HashSet::new())
}

fn expand_mods_impl(source: &str, base_dir: &Path, visited: &mut HashSet<String>) -> String {
    let mut result = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("mod \"") && trimmed.ends_with("\";") {
            let path = &trimmed[5..trimmed.len() - 2];
            let mod_path = base_dir.join(path);
            let canonical = mod_path
                .canonicalize()
                .unwrap_or_else(|_| mod_path.clone())
                .to_string_lossy()
                .to_string();
            if visited.contains(&canonical) {
                continue;
            }
            visited.insert(canonical);
            let mod_src = match std::fs::read_to_string(&mod_path) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("警告: mod 文件不存在: {}", mod_path.display());
                    continue;
                }
            };
            let mod_dir = mod_path.parent().unwrap_or(base_dir);
            result.push_str(&expand_mods_impl(&mod_src, mod_dir, visited));
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// 预处理器：提取 # 指令 + 收集 #define 名称 + 文本替换
pub fn extract_cpp_directives(source: &str) -> (String, Vec<(String, String)>, String) {
    let mut directives = String::new();
    let mut define_names = Vec::new();
    let mut expanded = source.to_string();

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
                    expanded = replace_ident(&expanded, &name, &value);
                    define_names.push((name, value));
                }
            }
        }
    }
    (directives, define_names, expanded)
}

/// 在源码中替换独立的标识符（前后不是字母/数字/下划线）
pub fn replace_ident(source: &str, name: &str, value: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let name_bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + name_bytes.len() <= bytes.len()
            && &bytes[i..i + name_bytes.len()] == name_bytes
            && (i == 0 || !is_ident_char(bytes[i - 1]))
            && (i + name_bytes.len() == bytes.len() || !is_ident_char(bytes[i + name_bytes.len()]))
        {
            result.push_str(value);
            i += name_bytes.len();
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

/// 判断字节是否为标识符字符（字母/数字/下划线）
pub fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
