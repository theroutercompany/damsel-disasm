#include <stdio.h>

static int helper(int value) {
    return value + 5;
}

int exported_sum(int lhs, int rhs) {
    return lhs + rhs + helper(1);
}

int main(void) {
    puts("hello from damsel");
    return exported_sum(2, 3) == 11 ? 0 : 1;
}
