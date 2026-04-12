#include <stdint.h>
#include <stdio.h>

static const char *lookup_kind(int value) {
    switch (value & 7) {
    case 0:
        return "zero";
    case 1:
        return "one";
    case 2:
        return "two";
    case 3:
        return "three";
    case 4:
        return "four";
    case 5:
        return "five";
    case 6:
        return "six";
    default:
        return "seven";
    }
}

int main(void) {
    volatile int seed = 5;
    const char *kind = lookup_kind(seed);
    printf("%s\n", kind);
    return (int)(uintptr_t)kind & 1;
}
