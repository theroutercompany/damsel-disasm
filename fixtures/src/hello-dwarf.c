#include <stdint.h>
#include <stdio.h>

typedef struct Point {
    int64_t x;
    int64_t y;
} Point;

typedef struct Shape {
    Point origin;
    uint32_t sides;
    uint32_t padding;
    char *label;
} Shape;

static int64_t area_of(const Shape *shape) {
    return shape->sides * (shape->origin.x + shape->origin.y);
}

int main(void) {
    Shape shape = { .origin = { 3, 4 }, .sides = 5, .padding = 0, .label = "pentagon" };
    printf("%s area: %lld\n", shape.label, area_of(&shape));
    return 0;
}
