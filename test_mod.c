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
void hello_a();
void hello_b();
int main();

#ifndef A_H
#define A_H 
void hello_a() {
    printf("A\n");
    return;
}

#endif
#ifndef B_H
#define B_H 
void hello_b() {
    printf("B\n");
    return;
}

#endif
int main() {
    hello_a();
    hello_b();
    return 0;
}

