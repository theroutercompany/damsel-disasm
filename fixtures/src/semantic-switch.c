#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int (*transform_fn)(int);

static int add_two(int value) {
    return value + 2;
}

static int mul_three(int value) {
    return value * 3;
}

static int xor_mask(int value) {
    return value ^ 0x5a;
}

static int rotate_left_one(int value) {
    return (value << 1) | ((value >> 30) & 1);
}

static transform_fn const TRANSFORMS[] = {
    add_two,
    mul_three,
    xor_mask,
    rotate_left_one,
};

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

static int dispatch_transform(int selector, int value) {
    transform_fn fn = TRANSFORMS[(unsigned)selector & 3u];
    return fn(value);
}

int main(int argc, char **argv) {
    volatile int seed = argc > 1 ? atoi(argv[1]) : 5;
    const char *kind = lookup_kind(seed);
    int transformed = dispatch_transform(seed, (int)strlen(kind));

    if ((seed & 1) == 0) {
        fprintf(stderr, "%s:%d\n", kind, transformed);
    } else {
        printf("%s:%d\n", kind, transformed);
    }

    return transformed & 7;
}
