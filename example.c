#include <stdint.h>
#include <stdio.h>

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

// === 函数声明 ===
int main();

// === 函数定义 ===
int main() {
    if (!1) {
        print(1);
    } else {
        print(0);
    }
    return 0;
}

