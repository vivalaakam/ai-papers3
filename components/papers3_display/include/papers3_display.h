#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int papers3_display_init(void);
int papers3_display_set_rotation(int rotation_degrees);
int papers3_display_set_font_size(int font_size);
int papers3_display_render_scene(
    const char* text,
    const uint8_t* bitmap,
    int bitmap_width,
    int bitmap_height,
    int bitmap_x,
    int bitmap_y,
    int x,
    int y,
    int width,
    int height
);
void papers3_display_deinit(void);

#ifdef __cplusplus
}
#endif
