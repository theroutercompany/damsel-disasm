#include <stdio.h>

int dup_shared_alpha(void) {
    return 41;
}

void dup_alpha_log(int value) {
    printf("alpha:%d\n", value);
}
