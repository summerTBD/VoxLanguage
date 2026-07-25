# Vox Language

A C-level systems language that compiles to C. Syntax inspired by Rust, semantics mapped 1:1 to C. Built-in Boehm GC with opt-out (`--no-gc`).

## Installation

### 从源码构建
```powershell
git clone https://github.com/summerTBD/VoxLanguage
cd VoxLanguage
cargo build --release
# 输出在 bin/ 目录
```

### 发布包（给别人用）
下载 `bin/` 整个文件夹，放到任意位置，加到系统 PATH。
需要 `gcc`（MinGW-w64）。

```
bin/
├── voxc.exe          # 编译器
├── voxlsp.exe        # LSP 服务器
├── vendor/           # Boehm GC 静态库
├── package.json      # VS Code 扩展清单
├── extension.js      # VS Code 扩展入口
├── syntaxes/         # 语法高亮
└── icon.png          # 图标
```

**VS Code 扩展安装**：把 `bin/` 目录创建 Junction 到 `~/.vscode/extensions/vox.vox-language-0.1.0`。

## Quick Start

```vox
// hello.vox
fn main() {
    puts("hello world");
}
```

```powershell
voxc hello.vox                  # → hello.c + hello.exe（同目录）
voxc hello.vox --out build      # → build/hello.c + build/hello.exe
voxc hello.vox --no-gc          # 不用 GC
voxc hello.vox --check          # 只检查，不编译
./hello.exe                     # → hello world
```

## Types

| Vox | C |
|-----|---|
| `i8` `i16` `i32` `i64` | `int8_t` `int16_t` `int32_t` `int64_t` |
| `u8` `u16` `u32` `u64` | `uint8_t` `uint16_t` `uint32_t` `uint64_t` |
| `f32` `f64` | `float` `double` |
| `char` | `char` |
| `bool` | `int` (C99 `_Bool` 兼容) |
| `str` | `const char*` |
| `*T` | `T*` |
| `[T; N]` | `T[N]` |

**隐式转换**：整数/浮点字面量可赋值给任意同族类型（`let a: i8 = 10;` ✅），变量间必须严格同类型或显式 `as`。

## Variables

```vox
let x: i32 = 42;           // 不可变
let mut y: i32 = 0;        // 可变
y = 100;
```

## Functions

```vox
fn add(a: i32, b: i32): i32 {
    return a + b;
}
```

**extern** — 调用 C 函数（prelude 自动提供 `puts` `getchar` `fopen` `fclose` `malloc` `free`）：

```vox
extern fn puts(s: str): i32;
```

**printf / scanf** — lexer 级整段透传，支持 `PRId32` 等 C 宏：

```vox
printf("x=%d, y=%f\n", x, y);
printf("i32 = %" PRId32 "\n", a);   // C 字符串拼接
scanf("%d", &x);
```

## Structs

```vox
struct Point {
    x: i32,
    y: i32,
}

let p: Point = Point { x: 10, y: 20 };   // 栈
let hp: *Point = new Point { x: 1, y: 2 }; // 堆（GC_malloc / malloc）
printf("p.x=%d, hp->x=%d\n", p.x, hp.x);
free(hp);  // --no-gc 时手动释放
```

## Enums + Match

```vox
enum Color {
    Red,        // 0
    Green = 5,  // 5
    Blue,       // 6
}

let c: Color = Color.Green;
match c {
    Red   -> { puts("red"); }
    Green -> { puts("green"); }
    Blue  -> { puts("blue"); }
}
```

## Control Flow

```vox
if x > 0 { ... }
else { ... }

while x < 10 { x = x + 1; }

for (let mut i: i32 = 0; i < 10; i = i + 1) {
    if i == 3 { continue; }
    if i == 7 { break; }
}
```

## Pointers & Arrays

```vox
let p: *i32 = &x;           // 取地址
let v: i32 = *p;            // 解引用
*p = 100;                   // 写入

let arr: [i32; 3] = [1, 2, 3];
let a: i32 = arr[0];        // 索引
arr[1] = 42;                // 写入

let q: *i32 = p + 1;        // 指针算术
let null_check: bool = p == 0;  // null 检查
```

## Misc

```vox
let x: i64 = 42 as i64;           // 类型转换
let sz: u64 = sizeof(Point);      // C sizeof
free(ptr);                        // 手动释放（--no-gc）
```

## Constants & Statics

```vox
const BUF: i32 = 4096;
static mut COUNTER: i32 = 0;
```

## C Macros

支持全部 C 预处理指令，作为一等语法，原位透传：

```vox
#ifndef VOX_H
#define VOX_H
#define BUF_SIZE 1024

fn main() {
    let arr: [i32; BUF_SIZE] = ...;
}
#endif
```

`#define` 名称自动做文本替换，`#ifndef`/`#endif` 保持原位保护代码。

## Modules

```vox
mod "utils.vox";    // 原地展开，支持嵌套
mod "math/vec.vox"; // 相对路径
```

## Memory Model

| 模式 | 分配 | 释放 |
|------|------|------|
| 默认 (GC) | `GC_malloc` | `GC_free` / 自动 |
| `--no-gc` | `malloc` | `free`（必须手动） |

```powershell
voxc app.vox                  # GC 模式
voxc --no-gc app.vox          # 裸指针
```

## Compiler Architecture

```
.vox 文件
  │
  ├─ mod 预处理器（展开 mod "..." + 提取 #define）
  ├─ prelude 注入
  ▼
Lexer → Tokens → Parser → AST → TypeChecker → Codegen → .c → gcc → .exe
```

```
src/
├── main.rs          # 入口，参数解析，gcc 调用
├── bin/voxlsp.rs    # LSP 语言服务器
├── lib.rs           # 模块声明
├── vox_ast.rs       # AST 定义（类型/表达式/语句/程序）
├── vox_lexer.rs     # 词法分析（Token 流）
├── vox_parser.rs    # 语法分析（Token → AST）
├── vox_token.rs     # Token 类型定义
├── vox_typeck.rs    # 类型检查
└── vox_codegen.rs   # AST → C 代码生成
```

## VS Code Extension

语法高亮 + LSP 错误标红。`Ctrl+Shift+B` 编译。

扩展文件在 `bin/` 目录（和 exe 同目录），LSP 调 `voxc --check` 保证和编译器 100% 一致。

## Requirements

- Rust (构建编译器)
- gcc (MinGW-w64 on Windows)

## License

MIT
