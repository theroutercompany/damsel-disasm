#include <stdio.h>

int dup_shared_beta(void) {
    return 73;
}

void dup_beta_log(int value) {
    printf("beta:%d\n", value);
}
