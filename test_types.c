#include <stdint.h>
#include <gc.h>

// === C 标准函数声明 ===
extern int printf(const char* fmt, ...);
extern int scanf(const char* fmt, ...);
extern int puts(const char* s);
extern int getchar(void);
extern void* fopen(const char* path, const char* mode);
extern int fclose(void* file);
extern int fprintf(void* file, const char* fmt, ...);

// === Vox 运行时 ===
static int32_t print(int32_t x) {
    printf("%d\n", x);
    return 0;
}

static int32_t read_i32() {
    int32_t x;
    scanf("%d", &x);
    return x;
}

static void print_f64(double x) {
    printf("%f\n", x);
}

static void print_str(const char* s) {
    printf("%s\n", s);
}

// === 函数声明 ===
int32_t puts(const char* s);
int32_t getchar();
void* fopen(const char* path, const char* mode);
int32_t fclose(void* file);
int32_t puts(const char* s);
int main();

// === 函数定义 ===
int main() {
    int8_t const a = 10;
    int16_t const b = 200;
    int32_t const c = 30000;
    int64_t const d = 4000000;
    uint8_t const e = 250;
    uint16_t const f = 60000;
    uint32_t const g = 4000000;
    uint64_t const h = 9999999;
    float const x = 3.14;
    double const y = 2.718;
    int8_t const ch = 65;
    int const ok = 1;
    int32_t arr[3] = { 1, 2, 3 };
    int32_t* const p = ((int32_t*)42);
    int32_t* const added = (p + 1);
    puts("All types OK!");
    print(42);
    return 0;
}

