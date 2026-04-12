#include <stdio.h>
#include <string.h>

static const char *decorate(const char *name, char *buffer, size_t size) {
    snprintf(buffer, size, "hello, %s (%zu)", name, strlen(name));
    return buffer;
}

int main(void) {
    char buffer[128];
    const char *message = decorate("damsel", buffer, sizeof(buffer));
    puts(message);
    fprintf(stderr, "stderr: %s\n", message);
    return strcmp(message, "hello, damsel (6)") == 0 ? 0 : 1;
}
