typedef int (*dispatch_fn)(int);

static int dispatch_alpha(int value) {
    return value + 10;
}

static int dispatch_beta(int value) {
    return value + 20;
}

static dispatch_fn TRANSFORMS[] = {
    dispatch_alpha,
    dispatch_beta,
};

int dispatch_value(int index, int value) {
    dispatch_fn fn = TRANSFORMS[index & 1];
    return fn(value);
}

int main(int argc, char **argv) {
    (void)argv;
    return dispatch_value(argc, argc + 1);
}
