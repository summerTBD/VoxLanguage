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

// === 枚举定义 ===
enum Color {
    Red = 0,
    Green = 1,
    Blue = 2,
};

// === 函数声明 ===
int32_t add(int32_t a, int32_t b);
int32_t sum_arr(int32_t* arr, int32_t len);
int main();

// === 函数定义 ===
int32_t add(int32_t a, int32_t b) {
    return (a + b);
}

int32_t sum_arr(int32_t* arr, int32_t len) {
    int32_t i = 0;
    int32_t s = 0;
    while ((i < len)) {
        s = (s + arr[i]);
        i = (i + 1);
    }
    return s;
}

int main() {
    int32_t nums[5] = { 1, 2, 3, 4, 5 };
    print(nums[0]);
    print(nums[4]);
    int32_t const total = sum_arr((&nums[0]), 5);
    print(total);
    enum Color const c = Green;
    switch (c) {
        case Red: {
            print(100);
            break;
        }
        case Green: {
            print(200);
            break;
        }
        case Blue: {
            print(300);
            break;
        }
    }
    struct Point* p = GC_malloc(sizeof(struct Point));
    p->x = 10;
    p->y = 20;
    print(add(p->x, p->y));
    return 0;
}

