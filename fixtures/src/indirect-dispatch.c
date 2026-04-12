#include <stdint.h>

typedef int (*dispatch_fn)(int);

static int dispatch_alpha(int value) {
    return value + 10;
}

static int dispatch_beta(int value) {
    return value + 20;
}

__attribute__((visibility("default")))
int exported_gamma(int value) {
    return value + 30;
}

static dispatch_fn volatile TRANSFORMS[] = {
    dispatch_alpha,
    dispatch_beta,
};

static uintptr_t volatile TARGETS[] = {
    (uintptr_t)&exported_gamma,
    (uintptr_t)&dispatch_beta,
};

int dispatch_value(int index, int value) {
    dispatch_fn fn = TRANSFORMS[index & 1];
    return fn(value);
}

int dispatch_second_slot(int value) {
    dispatch_fn volatile *table = TRANSFORMS;
    dispatch_fn fn = table[1];
    return fn(value);
}

uintptr_t load_export_target(void) {
    uintptr_t volatile *table = TARGETS;
    return table[0];
}

uintptr_t load_function_target(void) {
    uintptr_t volatile *table = TARGETS;
    return table[1];
}

int main(int argc, char **argv) {
    (void)argv;
    return dispatch_value(argc, argc + 1);
}
