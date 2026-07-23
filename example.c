#include <stdint.h>
#include <stdio.h>
#include <gc.h>

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
int32_t add(int32_t a, int32_t b);
int main();

// === 函数定义 ===
int32_t add(int32_t a, int32_t b) {
    return (a + b);
}

int main() {
    struct Point* p = GC_malloc(sizeof(struct Point));
    p->x = 10;
    p->y = 20;
    print(add(p->x, p->y));
    return 0;
}

