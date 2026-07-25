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
int main();

// === 函数定义 ===
int main() {
    int32_t const m = (10 % 3);
    printf("10 %% 3 = %d\n", m);
    int8_t const a = 10;
    int16_t const b = 200;
    int32_t const c = (a + b);
    printf("i8 + i16 = %d\n", c);
    if ((a < b)) {
        puts("a < b");
    }
    return 0;
}

