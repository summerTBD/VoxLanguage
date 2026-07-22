use vox_language::lexer::Lexer;

fn main() {
    let source = r#"
fn add(a: i32, b: i32): i32 {
    return a + b;
}

fn main() {
    let x: i32 = 10;
    let y: i32 = add(x, 20);
    print(y);
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    println!("=== Vox Lexer 输出 ===");
    for token in &tokens {
        println!("  第{}行 第{}列: {:?}", token.line, token.col, token.kind);
    }
    println!("=== 共 {} 个 Token ===", tokens.len());
}
