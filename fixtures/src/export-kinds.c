#include <stdint.h>

__attribute__((visibility("default")))
int exported_regular(int value) {
    return value + 1;
}

__attribute__((visibility("default"), weak))
int exported_weak(int value) {
    return value + 2;
}

__attribute__((visibility("default"))) __thread int exported_tls = 7;

static int use_exports(int value) {
    return exported_regular(value) + exported_weak(value) + exported_tls;
}

int main(void) {
    return use_exports(3);
}
