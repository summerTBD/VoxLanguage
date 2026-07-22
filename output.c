#include <stdint.h>
#include <stdio.h>

// === Vox 运行时 ===
static int32_t print(int32_t x) {
    printf("%d\n", x);
    return 0;
}

// === 函数声明 ===
int32_t add(int32_t a, int32_t b);
int main();

// === 函数定义 ===
int32_t add(int32_t a, int32_t b) {
    return (a + b);
}

int main() {
    int32_t const x = 10;
    int32_t const y = add(x, 5);
    print(y);
    return 0;
}

