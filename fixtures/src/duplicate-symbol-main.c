#include <stdio.h>

int dup_shared_alpha(void);
int dup_shared_beta(void);
void dup_alpha_log(int value);
void dup_beta_log(int value);

int main(void) {
    int left = dup_shared_alpha();
    int right = dup_shared_beta();
    dup_alpha_log(left);
    dup_beta_log(right);
    return (left ^ right) & 1;
}
