#include <stdint.h>
#include <gc.h>

// === C 运行时依赖 ===
extern int printf(const char* fmt, ...);
extern int scanf(const char* fmt, ...);
extern int puts(const char* s);

// === 函数声明 ===
int32_t puts(const char* s);
int32_t getchar();
void* fopen(const char* path, const char* mode);
int32_t fclose(void* file);
void* malloc(uint64_t size);
void free(void* ptr);
void noop();
int main();

void noop() {
    return;
    return;
}

int main() {
    noop();
    int32_t arr[4] = { 1, 2, 3, 4 };
    int32_t* const p = (&arr[0]);
    int32_t* const q = (p + 2);
    printf("*q = %d\n", (*q));
    printf("10 %% 3 = %d\n", (10 % 3));
    return 0;
}

