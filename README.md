# Vox Language

A C-level systems language that compiles to C. Syntax inspired by Rust, semantics mapped 1:1 to C. Built-in Boehm GC with opt-out (`--no-gc`).

---

## 1. 项目概览

### 设计哲学

Vox 是一个**对 C 零阻抗**的系统语言。不引入虚拟机、不隐藏内存模型。每个 Vox 表达式直接映射到 C 表达式。目标：用更现代的语法写 C，同时保留 C 的全部能力（指针算术、C 宏、标准库）。

### 核心原则

1. **1:1 C 映射** — `i32` = `int32_t`，`*T` = `T*`，`[T; N]` = `T[N]`。没有隐式 box、没有 vtable、没有胖指针。
2. **C 是一等公民** — 12 个预处理关键字（`#define`/`#ifndef` 等）是原生语法，`printf`/`scanf` 整段透传支持 PRId32 等宏。
3. **GC 可选** — 默认 `GC_malloc`/`GC_free`（Boehm），`--no-gc` 切到 `malloc`/`free`。
4. **自举友好** — AST 结构简单，编译管线线性，生成纯 C 不依赖额外 runtime。

---

## 2. 快速开始

```powershell
# 构建编译器
git clone https://github.com/summerTBD/VoxLanguage
cd VoxLanguage
cargo build --release

# 编译 Vox 程序
./target/release/vox-language.exe hello.vox                  # → hello.c + hello.exe
./target/release/vox-language.exe hello.vox --out build      # → build/hello.c + build/hello.exe
./target/release/vox-language.exe hello.vox --no-gc          # 不用 GC，malloc/free
./target/release/vox-language.exe hello.vox --check          # 只类型检查，不编译
```

---

## 3. 编译管线（Compiler Pipeline）

```
输入 .vox 源码
  │
  ├─ main.rs: read_to_string
  ├─ lib.rs: expand_mods()        — 展开 mod "file.vox"（递归 + visited 去重）
  ├─ lib.rs: replace_defines()    — 收集 #define 名称，文本替换（跳过字符串值 + #define 行自身）
  ├─ main.rs: include_str!("../prelude.vox")  — 拼接 prelude 到源码顶部
  ▼
Lexer (vox_lexer.rs)
  └─ 逐字符扫描 → Token 流（TokenKind + 行列号）
     ├─ C 预处理指令 → read_cpp_directive() → CppLine
     ├─ printf/scanf → read_balanced_parens() → Printf(String)/Scanf(String)
     └─ 标识符匹配 20+ 关键字
  ▼
Parser (vox_parser.rs)
  └─ 递归下降 → Program { items: Vec<TopLevelItem> }
     ├─ TopLevelItem: CppLine | Struct | Enum | Function | Const | Static
     └─ 按源文件顺序保存（C 宏和声明顺序不丢失）
  ▼
TypeChecker (vox_typeck.rs)
  └─ Program 遍历 → 类型推导 + 错误报告
     ├─ register_defines() — #define 名称标记为已知标识符
     ├─ 跨宽度算术 → wider() 自动提升
     ├─ 字面量 → 整数族/浮点族隐式转换
     └─ Printf/Scanf → 返回 Int(Signed, 32)，不检查参数
  ▼
Codegen (vox_codegen.rs)
  └─ Program.items 顺序遍历 → C 源码字符串
     ├─ #include <stdint.h> <inttypes.h> <stdio.h> <stdlib.h> <string.h>
     ├─ extern fn → 跳过 C 声明（标准头文件提供）
     ├─ CppLine → 原样输出
     └─ 表达式编译 → compile_expr() / compile_stmt()
  ▼
gcc (MinGW-w64)
  └─ gcc hello.c -o hello.exe [-I vendor/include vendor/libgc.a]
```

### 关键数据类型

```
TokenKind     — 20+ 关键字 + 标点 + Printf(String) + Scanf(String)
TopLevelItem  — CppLine | Struct | Enum | Function | Const | Static
Type          — Int(Signedness, u8) | Float(u8) | Bool | Char | Str | Void | Ptr | Array | Adt
Expression    — IntLiteral | FloatLiteral | StringLiteral | Identifier | Binary | Call | 
                StructLiteral | FieldAccess | Not | New | AddrOf | Deref | Cast | Sizeof |
                ArrayLiteral | Index | Printf | Scanf
Statement     — Let | Return | Expr | If | While | Assign | Store | StoreField | StoreIndex |
                Match | For | Break | Continue | Define | CppDirective
```

---

## 4. C 预处理指令（一等语法）

12 个宏关键字在 lexer 层识别，parser 层作为 `CppLine(String)` 保存，codegen 原样透传：

```vox
#define      #undef       #include
#ifdef       #ifndef      #if
#else        #elif        #endif
#pragma      #error       #line
```

`#define` 名称还会在 `replace_defines()` 中做文本替换（两遍扫描），这样 Vox 代码中可直接使用宏名作为数组大小、常量值等。

---

## 5. printf / scanf 透传

Lexer 在遇到标识符 `printf` 或 `scanf` 后紧跟 `(` 时，调用 `read_balanced_parens()` 整段捕获直到匹配的 `)`，生成 `Printf(String)` / `Scanf(String)` token。Parser 将其作为 `Expression::Printf` / `Expression::Scanf`，Codegen 直接输出 `printf(...)` / `scanf(...)`。

这支持 C 字符串拼接——`PRId32` 来自 `<inttypes.h>`，无需在 Vox 侧定义：

```vox
printf("i32 = %" PRId32 "\n", a);   // → C: printf("i32 = %" PRId32 "\n", a);
```

---

## 6. prelude.vox 与标准头文件

编译器自动 `#include` 五个标准头文件：

| 头文件 | 提供 |
|--------|------|
| `<stdint.h>` | `int8_t` `uint64_t` 等整型 typedef |
| `<inttypes.h>` | `PRId8` `PRIu64` 等 printf 格式宏 |
| `<stdio.h>` | `FILE*` `printf` `scanf` `fopen` `fread` ... |
| `<stdlib.h>` | `malloc` `free` `atoi` `exit` ... |
| `<string.h>` | `memcpy` `memset` `strlen` `strcmp` ... |

### extern fn 规则

`prelude.vox` 中所有 `extern fn` 声明**只用于 Vox 类型检查**，codegen 不会为其生成 C 前向声明（因为标准头文件已提供），避免签名冲突（如 `fopen` 在 Vox 中声明为 `*void`，而 `stdio.h` 中为 `FILE*`）。

用户自定义的 `extern fn` 同样不会生成 C 声明——需用户自行 `#include` 对应头文件。

### prelude 提供的内置函数

**stdio.h**: `puts` `getchar` `fopen` `fclose` `fgetc` `fputc` `fgets` `fputs` `fread` `fwrite` `fseek` `ftell` `rewind` `fflush` `feof` `ferror` `perror` `remove` `rename`

**stdlib.h**: `malloc` `calloc` `realloc` `free` `atoi` `atol` `atof` `strtol` `strtoul` `strtod` `abs` `rand` `srand` `system` `exit`

**string.h**: `memcpy` `memmove` `memset` `memcmp` `memchr` `strlen` `strcpy` `strncpy` `strcat` `strncat` `strcmp` `strncmp` `strcoll` `strchr` `strrchr` `strspn` `strcspn` `strpbrk` `strstr` `strtok` `strerror`

---

## 7. 类型系统

| Vox 类型 | C 类型 | 说明 |
|----------|--------|------|
| `i8` `i16` `i32` `i64` | `int8_t` ... `int64_t` | 有符号整数 |
| `u8` `u16` `u32` `u64` | `uint8_t` ... `uint64_t` | 无符号整数 |
| `f32` `f64` | `float` `double` | 浮点 |
| `char` | `char` | 字符 |
| `bool` | `int` | 布尔（C 兼容） |
| `str` | `const char*` | 字符串（不可变 C 字符串） |
| `void` | `void` | 空类型 |
| `*T` | `T*` | 指针 |
| `[T; N]` | `T[N]` | 定长数组 |

### 隐式转换规则（typeck）

- **字面量 → 同族任意宽度**：`let a: i8 = 10;` ✅，`let b: f32 = 3.14;` ✅
- **整数跨宽度运算**：`i32 + u64 → u64`（`wider()` 自动提升）
- **整数 + 浮点**：暂不自动转换，需显式 `as`
- **变量间**：必须严格类型匹配

---

## 8. 语言参考

### 变量

```vox
let x: i32 = 42;           // 不可变
let mut y: i32 = 0;        // 可变
y = 100;
```

### 函数

```vox
fn add(a: i32, b: i32): i32 {
    return a + b;
}

extern fn my_c_func(x: i32): *void;   // C 函数，不生成声明
```

### 结构体

```vox
struct Point {
    x: i32,
    y: i32,
}

let p: Point = Point { x: 10, y: 20 };     // 栈分配
let hp: *Point = new Point { x: 1, y: 2 }; // 堆分配（GC_malloc 或 malloc）
p.x;  hp.x;  // 字段访问（栈用点，堆也用点，codegen 自动加 ->）
```

### 枚举 + Match

```vox
enum Color { Red, Green = 5, Blue }  // Red=0, Green=5, Blue=6
let c: Color = Color.Green;
match c {
    Red   -> { puts("red"); }
    Green -> { puts("green"); }
    Blue  -> { puts("blue"); }
}
```

### 控制流

```vox
if x > 0 { ... } else { ... }

while x < 10 { x = x + 1; }

for (let mut i: i32 = 0; i < 10; i = i + 1) {
    if i == 3 { continue; }
    if i == 7 { break; }
}
```

### 指针与数组

```vox
let p: *i32 = &x;          // 取地址 → C: &x
let v: i32 = *p;           // 解引用 → C: *p
*p = 100;                  // 写入 → C: *p = 100;

let arr: [i32; 3] = [1, 2, 3];
arr[0];  arr[1] = 42;      // 索引

p + 1;  p == 0;            // 指针算术，null 检查
```

### 其他

```vox
42 as i64                  // 类型转换 → C: (int64_t)42
sizeof(Point)              // → C: sizeof(struct Point)
const BUF: i32 = 4096;     // → C: static const int32_t BUF = 4096;
static mut COUNTER: i32 = 0; // → C: static int32_t COUNTER = 0;
```

### 模块

```vox
mod "utils.vox";           // 相对路径展开，支持嵌套，自动去重
mod "math/vec.vox";
```

---

## 9. GC 与内存模型

| 编译选项 | `new` 展开为 | 释放 |
|----------|-------------|------|
| 默认（GC） | `GC_malloc(sizeof(T))` | `GC_free` / 自动回收 |
| `--no-gc` | `malloc(sizeof(T))` | 必须手动 `free(ptr)` |

GC 编译时 gcc 额外链接 `vendor/include`（头文件）和 `vendor/libgc.a`（Boehm GC 静态库）。

---

## 10. 源代码结构

```
VoxLanguage/
├── Cargo.toml              # Rust 项目配置（default-run = vox-language）
├── prelude.vox             # C 标准函数声明（extern fn，仅供类型检查）
├── example.vox             # 示例程序
├── README.md               # 本文档
│
├── src/
│   ├── main.rs             # 入口：参数解析 + expand_mods + replace_defines + prelude 注入
│   │                       #       Lexer → Parser → TypeChecker → Codegen → gcc → run
│   ├── lib.rs              # expand_mods() replace_defines() replace_ident() is_ident_char()
│   ├── vox_token.rs        # TokenKind enum（关键字 + 标点 + Printf/Scanf）
│   ├── vox_lexer.rs        # Lexer: next_token() 主循环 + read_cpp_directive() + read_balanced_parens()
│   ├── vox_ast.rs          # AST: TopLevelItem Program Type Expression Statement StructDef EnumDef ...
│   ├── vox_parser.rs       # Parser: parse_program() 递归下降，format_cpp_directive()
│   ├── vox_typeck.rs       # TypeChecker: register_defines() check() wider() 字面量隐式转换
│   ├── vox_codegen.rs      # Codegen: compile() emit_function_decl() compile_expr() compile_stmt()
│   │                       #          type_to_c() ret_type_to_c() alloc_fn() free_fn()
│   └── bin/
│       └── voxlsp.rs       # LSP: JSON-RPC over stdin/stdout，调 voxc --check 子进程
│
├── bin/                    # 发布目录（VS Code 扩展 + exe）
│   ├── package.json        # VS Code 扩展清单
│   ├── extension.js        # 扩展入口（调 ./voxlsp.exe）
│   ├── syntaxes/
│   │   └── vox.tmLanguage.json  # TextMate 语法高亮
│   └── icon.png
│
└── vendor/                 # Boehm GC（voxc.exe 同级目录）
    ├── include/            # gc.h ...
    └── libgc.a
```

---

## 11. 关键实现细节（自举须知）

### Lexer 特殊行为

- **`#` 开头** → `read_cpp_directive()` 读到换行，匹配 12 个宏指令名
- **标识符 `printf` / `scanf` + `(`** → `read_balanced_parens()` 整段透传
- **数字字面量** → 支持 `0x` 前缀十六进制，`.` 判浮点
- **字符串** → 双引号，支持 `\"` `\\` `\n` `\t` 转义
- **注释** → 仅 `//` 行注释

### Parser 特殊行为

- **`parse_program()`** 顶层循环：检查 `#` → `CppLine`，检查 `struct`/`enum`/`fn`/`const`/`static`/`extern` → 对应 AST 节点
- **`#define`** → `Statement::Define`（名称+值），其他 `#` → `Statement::CppDirective`（整行）
- **`.` 字段访问与 `->`**：Vox 统一用 `.`，codegen 自动判断指针类型加 `->`
- **`new Struct { ... }`** → `Expression::New`，codegen 生成 `alloc_fn()(sizeof(struct T))`
- **`return;`** 无值 → `Statement::Return(None)`

### TypeChecker 特殊行为

- **`#define` 名称** → `register_defines()` 注册到 `self.defines`，类型检测时通过
- **`Expression::Printf` / `Expression::Scanf`** → 返回 `Int(Signed, 32)`
- **跨宽度算术** → `Type::wider()`：取最大位宽，有符号+无符号=无符号
- **字面量隐式转换** → 仅整数族和浮点族各自内部允许

### Codegen 特殊行为

- **extern fn** → 不生成 C 前向声明（标准头文件提供）
- **main 函数** → 返回类型强制 `int`（C 标准要求）
- **struct 字段访问** → 检测 `Type::Ptr`，自动 `->`，否则 `.`
- **`new T { ... }`** → `(struct T*)alloc_fn()(sizeof(struct T))` + 逐字段赋值
- **头文件顺序** → `stdint.h` `inttypes.h` `stdio.h` `stdlib.h` `string.h` [+ `gc.h`]

### main.rs 编译流程

1. `expand_mods(source, base_dir)` — mod 展开 + visited 去重
2. `replace_defines(source)` — 两遍扫描：收集名称 → 替换（跳过 `#define` 行和字符串值）
3. `include_str!("../prelude.vox")` — 拼接 prelude
4. `Lexer::new()` + `Parser::new()` → `Program`
5. `TypeChecker::new()` + `register_defines()` + `check()`
6. 如果 `--check` → `exit(0)`
7. `Codegen::new(!no_gc)` + `compile()` → C 源码
8. `std::fs::write(c_path, c_code)`
9. `gcc c_path -o exe_path [-I vendor/include vendor/libgc.a]`
10. 运行 exe 并打印输出

### LSP (voxlsp.rs)

- 通过标准输入/输出 JSON-RPC 通信
- `textDocument/didChange` / `didOpen` / `didSave` → 写临时文件到源文件目录 → `voxc --check tmp.vox` 子进程
- 解析 stderr 中的错误信息，转换行列号（Vox 报的是 prelude 注入后的行号，需减去 prelude 行数）
- URL 解码（`%3A` → `:`）处理 Windows 盘符路径

---

## 12. 构建与发布

### 开发构建

```powershell
cargo build                    # debug 构建 → target/debug/
cargo build --release          # release 构建 → target/release/
```

### 发布包

```powershell
# bin/ 目录结构（给用户）
bin/
├── voxc.exe              # 编译器（或 vox-language.exe）
├── voxlsp.exe            # LSP
├── vendor/include/       # gc.h
├── vendor/libgc.a        # Boehm GC 静态库
├── package.json          # VS Code 扩展清单
├── extension.js          # 扩展入口
├── syntaxes/vox.tmLanguage.json
└── icon.png
```

### VS Code 扩展

将 `bin/` 目录创建 Junction 到 VS Code 扩展目录：

```powershell
New-Item -ItemType Junction -Path "$env:USERPROFILE\.vscode\extensions\vox.vox-language-0.1.0" -Target "D:\MyProjects\VoxLanguage\bin"
```

扩展仅 8 行 JS，通过 `__dirname` 定位 `voxlsp.exe`，零配置。

---

## 13. 已知限制

- **无泛型** — `Vec<T>` 等需要单态化支持（AST 已预留 `Type::Adt { args }` 字段）
- **无 variadic** — 仅 `printf`/`scanf` 特殊支持，`fprintf`/`sprintf` 暂不可用
- **无闭包/lambda**
- **无 trait/interface**
- **match 仅支持枚举** — 不支持整数/字符串匹配
- **for 循环仅 C 风格** — 无 `for x in iter`
- **结构体全堆分配** — `new Struct { }` 始终调 `alloc_fn()`，无栈 `new`
- **bool = int** — 未使用 C99 `_Bool`/`stdbool.h`
- **fopen 返回 `*void`** — prelude 中声明为 `*void` 而非 `FILE*`，实际使用需强转

---

## 14. 依赖

- **Rust** 1.70+ — 编译器构建
- **gcc** (MinGW-w64 on Windows) — C 编译
- **Boehm GC** — `vendor/` 中的预编译静态库（可选，`--no-gc` 可跳过）

## License

MIT
