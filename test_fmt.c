#include <stdint.h>
#include <inttypes.h>
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

#define PRId8 "hhd"
#define PRId16 "hd"
#define PRId32 "d"
#define PRId64 "lld"
#define PRIu8 "hhu"
#define PRIu16 "hu"
#define PRIu32 "u"
#define PRIu64 "llu"
int main() {
    int32_t const a = 42;
    printf("i32 = %" PRId32 "\n", a);
    return 0;
}

