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
    const char* const s = "hello";
    puts(s);
    printf("pow: %d\n", (2 * 2));
    return 0;
}

