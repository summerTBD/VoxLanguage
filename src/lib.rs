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
                    // 文件不存在（LSP 下模块可能尚未创建），跳过
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
