#include <stdio.h>

static int helper(int value) {
    int scaled = value * 3;
    return scaled + 7;
}

int exported_sum(int lhs, int rhs) {
    return lhs + rhs + helper(2);
}

int main(void) {
    puts("hello from damsel");
    return exported_sum(2, 3) == 12 ? 0 : 1;
}
