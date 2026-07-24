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

// === 结构体定义 ===
struct Point {
    int32_t x;
    int32_t y;
};

// === 函数声明 ===
int32_t puts(const char* s);
int32_t getchar();
void* fopen(const char* path, const char* mode);
int32_t fclose(void* file);
int32_t puts(const char* s);
int main();

// === 函数定义 ===
int main() {
    struct Point* p = GC_malloc(sizeof(struct Point));
    p->x = 10;
    p->y = 20;
    print(p->x);
    GC_free(p);
    puts("done");
    return 0;
}

