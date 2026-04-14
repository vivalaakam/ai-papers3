#include "papers3_display.h"
#include "papers3_font.h"
#include "ubuntu_font_14.h"
#include "ubuntu_font_16.h"
#include "ubuntu_font_18.h"
#include "ubuntu_font_20.h"
#include "ubuntu_font_22.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include <esp_log.h>
#include <esp_heap_caps.h>
#include <freertos/FreeRTOS.h>
#include <freertos/task.h>

#include <epdiy.h>

#ifndef EPD_ROT_INVERTED_PORTRAIT
#define EPD_ROT_INVERTED_PORTRAIT 2
#endif

#ifndef EPD_ROT_INVERTED_LANDSCAPE
#define EPD_ROT_INVERTED_LANDSCAPE 3
#endif

static const char* TAG = "papers3_display";
static EpdiyHighlevelState s_highlevel;
static bool s_initialized = false;
static int s_rotation = EPD_ROT_INVERTED_PORTRAIT;
static const FontFace* s_active_font = &ubuntu_16_font;
static uint8_t* s_framebuffer = NULL;
static uint8_t* s_scene_buffer = NULL;
static uint8_t* s_prev_buffer = NULL;
static size_t s_buffer_size = 0;
static int s_display_width = 0;
static int s_display_height = 0;
static int s_logical_width = 0;
static int s_logical_height = 0;
static bool s_has_prev_frame = false;

static void draw_bitmap(
    uint8_t* framebuffer,
    const uint8_t* bitmap,
    int bitmap_width,
    int bitmap_height,
    int bitmap_x,
    int bitmap_y
);
static int check_draw_error(enum EpdDrawError err);
static int ensure_buffers(void);
static void clear_buffer(uint8_t* buffer);
static void update_dirty_bounds(int x, int y, int* min_x, int* min_y, int* max_x, int* max_y);
static int render_buffer(const uint8_t* buffer);
static void map_physical_to_logical(int phys_x, int phys_y, int* out_x, int* out_y);

static const FontGlyph* find_glyph(uint16_t codepoint) {
    for (uint32_t i = 0; i < s_active_font->glyph_count; i++) {
        if (s_active_font->glyphs[i].codepoint == codepoint) {
            return &s_active_font->glyphs[i];
        }
    }

    return NULL;
}

static const FontGlyph* resolve_glyph(uint16_t codepoint) {
    const FontGlyph* glyph = find_glyph(codepoint);
    if (glyph != NULL) {
        return glyph;
    }

    glyph = find_glyph('?');
    if (glyph != NULL) {
        return glyph;
    }

    return find_glyph(' ');
}

static void draw_glyph(uint8_t* framebuffer, int x, int baseline_y, const FontGlyph* glyph, uint8_t color) {
    if (glyph == NULL) {
        return;
    }

    int bytes_per_row = (glyph->width + 7) / 8;
    const uint8_t* bitmap = s_active_font->bitmap + glyph->bitmap_offset;
    int start_x = x + glyph->x_offset;
    int start_y = baseline_y + glyph->y_offset;

    for (int row = 0; row < glyph->height; row++) {
        const uint8_t* row_ptr = bitmap + (row * bytes_per_row);
        for (int col = 0; col < glyph->width; col++) {
            if (row_ptr[col / 8] & (0x80 >> (col % 8))) {
                epd_draw_pixel(start_x + col, start_y + row, color, framebuffer);
            }
        }
    }
}

static const char* decode_utf8(const char* text, uint16_t* codepoint) {
    unsigned char first = (unsigned char)text[0];
    if (first < 0x80) {
        *codepoint = first;
        return text + 1;
    }

    if ((first & 0xE0) == 0xC0) {
        unsigned char second = (unsigned char)text[1];
        if ((second & 0xC0) == 0x80) {
            *codepoint = (uint16_t)(((first & 0x1F) << 6) | (second & 0x3F));
            return text + 2;
        }
    } else if ((first & 0xF0) == 0xE0) {
        unsigned char second = (unsigned char)text[1];
        unsigned char third = (unsigned char)text[2];
        if (((second & 0xC0) == 0x80) && ((third & 0xC0) == 0x80)) {
            *codepoint = (uint16_t)(((first & 0x0F) << 12) | ((second & 0x3F) << 6) | (third & 0x3F));
            return text + 3;
        }
    }

    *codepoint = '?';
    return text + 1;
}

static void draw_text(uint8_t* framebuffer, int x, int y, const char* text, uint8_t color) {
    int cursor_x = x;
    int cursor_y = y + s_active_font->ascent;

    for (const char* p = text; *p != '\0';) {
        uint16_t codepoint = 0;
        const char* next = decode_utf8(p, &codepoint);

        if (codepoint == '\n') {
            cursor_x = x;
            cursor_y += s_active_font->line_height;
            p = next;
            continue;
        }

        const FontGlyph* glyph = resolve_glyph(codepoint);
        if (glyph != NULL) {
            draw_glyph(framebuffer, cursor_x, cursor_y, glyph, color);
            cursor_x += glyph->x_advance;
        } else {
            cursor_x += s_active_font->size / 2;
        }
        p = next;
    }
}

static int ensure_initialized(void) {
    if (!s_initialized) {
        int init_result = papers3_display_init();
        if (init_result != 0) {
            return init_result;
        }
    }

    return 0;
}

static int ensure_buffers(void) {
    if (!s_initialized) {
        return -1;
    }

    int physical_width = epd_width();
    int physical_height = epd_height();
    int logical_width = epd_rotated_display_width();
    int logical_height = epd_rotated_display_height();
    if (physical_width <= 0 || physical_height <= 0 || logical_width <= 0 || logical_height <= 0) {
        ESP_LOGE(
            TAG,
            "invalid display size: physical %dx%d logical %dx%d",
            physical_width,
            physical_height,
            logical_width,
            logical_height
        );
        return -1;
    }

    size_t bytes_per_row = (size_t)(physical_width + 1) / 2;
    size_t required_size = bytes_per_row * (size_t)physical_height;
    if (s_scene_buffer != NULL && s_prev_buffer != NULL && s_buffer_size == required_size) {
        s_display_width = physical_width;
        s_display_height = physical_height;
        s_logical_width = logical_width;
        s_logical_height = logical_height;
        return 0;
    }

    if (s_scene_buffer != NULL) {
        heap_caps_free(s_scene_buffer);
        s_scene_buffer = NULL;
    }
    if (s_prev_buffer != NULL) {
        heap_caps_free(s_prev_buffer);
        s_prev_buffer = NULL;
    }

    s_scene_buffer = (uint8_t*)heap_caps_malloc(required_size, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
    s_prev_buffer = (uint8_t*)heap_caps_malloc(required_size, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
    if (s_scene_buffer == NULL || s_prev_buffer == NULL) {
        ESP_LOGE(TAG, "failed to allocate framebuffers (%zu bytes)", required_size);
        if (s_scene_buffer != NULL) {
            heap_caps_free(s_scene_buffer);
            s_scene_buffer = NULL;
        }
        if (s_prev_buffer != NULL) {
            heap_caps_free(s_prev_buffer);
            s_prev_buffer = NULL;
        }
        return -1;
    }

    s_buffer_size = required_size;
    s_display_width = physical_width;
    s_display_height = physical_height;
    s_logical_width = logical_width;
    s_logical_height = logical_height;
    clear_buffer(s_scene_buffer);
    clear_buffer(s_prev_buffer);
    s_has_prev_frame = false;
    return 0;
}

static void clear_buffer(uint8_t* buffer) {
    if (buffer == NULL || s_buffer_size == 0) {
        return;
    }
    memset(buffer, 0xFF, s_buffer_size);
}

static void update_dirty_bounds(int x, int y, int* min_x, int* min_y, int* max_x, int* max_y) {
    if (x < *min_x) {
        *min_x = x;
    }
    if (y < *min_y) {
        *min_y = y;
    }
    if (x > *max_x) {
        *max_x = x;
    }
    if (y > *max_y) {
        *max_y = y;
    }
}

int papers3_display_begin(void) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }

    if (ensure_buffers() != 0) {
        return -1;
    }

    s_framebuffer = epd_hl_get_framebuffer(&s_highlevel);
    clear_buffer(s_scene_buffer);
    return 0;
}

int papers3_display_draw_text(const char* text, int x, int y) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }

    if (ensure_buffers() != 0) {
        return -1;
    }

    if (s_scene_buffer == NULL) {
        return -1;
    }

    draw_text(s_scene_buffer, x, y, text, 0x00);
    return 0;
}

int papers3_display_draw_bitmap(
    const uint8_t* bitmap,
    int bitmap_width,
    int bitmap_height,
    int bitmap_x,
    int bitmap_y
) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }

    if (bitmap == NULL || bitmap_width <= 0 || bitmap_height <= 0) {
        return 0;
    }

    if (ensure_buffers() != 0) {
        return -1;
    }

    draw_bitmap(
        s_scene_buffer,
        bitmap,
        bitmap_width,
        bitmap_height,
        bitmap_x,
        bitmap_y
    );

    return 0;
}

int papers3_display_draw_rect(
    int x,
    int y,
    int width,
    int height,
    int fill_color,
    int stroke_color
) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }

    if (ensure_buffers() != 0) {
        return -1;
    }

    EpdRect rect = {
        .x = x,
        .y = y,
        .width = width,
        .height = height,
    };

    if (fill_color >= 0) {
        epd_fill_rect(rect, (uint8_t)fill_color, s_scene_buffer);
    }

    if (stroke_color >= 0) {
        epd_draw_rect(rect, (uint8_t)stroke_color, s_scene_buffer);
    }

    return 0;
}

int papers3_display_commit(void) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }

    return render_buffer(s_scene_buffer);
}

static void draw_bitmap(
    uint8_t* framebuffer,
    const uint8_t* bitmap,
    int bitmap_width,
    int bitmap_height,
    int bitmap_x,
    int bitmap_y
) {
    if (bitmap == NULL || bitmap_width <= 0 || bitmap_height <= 0) {
        return;
    }

    for (int row = 0; row < bitmap_height; row++) {
        for (int col = 0; col < bitmap_width; col++) {
            epd_draw_pixel(
                bitmap_x + col,
                bitmap_y + row,
                bitmap[row * bitmap_width + col],
                framebuffer
            );
        }
    }
}

static int check_draw_error(enum EpdDrawError err) {
    if (err != EPD_DRAW_SUCCESS) {
        ESP_LOGE(TAG, "draw error: 0x%x", err);
        return (int)err;
    }

    return 0;
}

int papers3_display_init(void) {
    if (s_initialized) {
        return 0;
    }

    epd_init(&epd_board_m5papers3, &ED047TC2, EPD_LUT_64K);
    s_highlevel = epd_hl_init(EPD_BUILTIN_WAVEFORM);
    epd_set_rotation(s_rotation);

    epd_poweron();
    epd_clear();
    vTaskDelay(pdMS_TO_TICKS(500));
    epd_clear();
    epd_poweroff();

    s_initialized = true;
    return 0;
}

int papers3_display_width(void) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }
    if (ensure_buffers() != 0) {
        return -1;
    }
    return s_logical_width;
}

int papers3_display_height(void) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }
    if (ensure_buffers() != 0) {
        return -1;
    }
    return s_logical_height;
}

int papers3_display_physical_width(void) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }
    if (ensure_buffers() != 0) {
        return -1;
    }
    return s_display_width;
}

int papers3_display_physical_height(void) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }
    if (ensure_buffers() != 0) {
        return -1;
    }
    return s_display_height;
}

int papers3_display_get_rotation(void) {
    return s_rotation;
}

int papers3_display_present(const uint8_t* buffer, int width, int height) {
    int init_result = ensure_initialized();
    if (init_result != 0) {
        return init_result;
    }

    if (buffer == NULL) {
        return -1;
    }

    if (ensure_buffers() != 0) {
        return -1;
    }

    if (width != s_display_width || height != s_display_height) {
        ESP_LOGE(
            TAG,
            "buffer size mismatch: got %dx%d expected %dx%d",
            width,
            height,
            s_display_width,
            s_display_height
        );
        return -1;
    }

    return render_buffer(buffer);
}

int papers3_display_set_font_size(int font_size) {
    switch (font_size) {
        case 18:
            s_active_font = &ubuntu_18_font;
            break;
        case 14:
            s_active_font = &ubuntu_14_font;
            break;
        case 16:
            s_active_font = &ubuntu_16_font;
            break;
        case 20:
            s_active_font = &ubuntu_20_font;
            break;
        case 22:
            s_active_font = &ubuntu_22_font;
            break;
        default:
            return -1;
    }

    return 0;
}

int papers3_display_set_rotation(int rotation_degrees) {
    int normalized = rotation_degrees % 360;
    if (normalized < 0) {
        normalized += 360;
    }

    switch (normalized) {
        case 0:
            s_rotation = EPD_ROT_PORTRAIT;
            break;
        case 90:
            s_rotation = EPD_ROT_LANDSCAPE;
            break;
        case 180:
            s_rotation = EPD_ROT_INVERTED_PORTRAIT;
            break;
        case 270:
            s_rotation = EPD_ROT_INVERTED_LANDSCAPE;
            break;
        default:
            return -1;
    }

    if (s_initialized) {
        epd_set_rotation(s_rotation);
        if (ensure_buffers() != 0) {
            return -1;
        }
    }

    return 0;
}

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
) {
    int init_result = papers3_display_begin();
    if (init_result != 0) {
        return init_result;
    }

    EpdRect rect = {
        .x = x,
        .y = y,
        .width = width,
        .height = height,
    };

    epd_fill_rect(rect, 0xD0, s_framebuffer);
    epd_draw_rect(rect, 0x00, s_framebuffer);

    draw_text(s_framebuffer, x + 16, y + 16, text, 0x00);
    draw_bitmap(
        s_framebuffer,
        bitmap,
        bitmap_width,
        bitmap_height,
        bitmap_x,
        bitmap_y
    );
    return papers3_display_commit();
}

void papers3_display_deinit(void) {
    if (!s_initialized) {
        return;
    }

    epd_deinit();
    s_initialized = false;
    s_framebuffer = NULL;
    if (s_scene_buffer != NULL) {
        heap_caps_free(s_scene_buffer);
        s_scene_buffer = NULL;
    }
    if (s_prev_buffer != NULL) {
        heap_caps_free(s_prev_buffer);
        s_prev_buffer = NULL;
    }
    s_buffer_size = 0;
    s_display_width = 0;
    s_display_height = 0;
    s_logical_width = 0;
    s_logical_height = 0;
    s_has_prev_frame = false;
}

static int render_buffer(const uint8_t* buffer) {
    if (buffer == NULL) {
        return -1;
    }

    if (ensure_buffers() != 0) {
        return -1;
    }

    if (s_framebuffer == NULL) {
        s_framebuffer = epd_hl_get_framebuffer(&s_highlevel);
    }

    if (!s_has_prev_frame) {
        memcpy(s_framebuffer, buffer, s_buffer_size);
        epd_poweron();
        int temperature = (int)epd_ambient_temperature();
        int err = check_draw_error(epd_hl_update_screen(&s_highlevel, MODE_GC16, temperature));
        epd_poweroff();
        memcpy(s_prev_buffer, buffer, s_buffer_size);
        s_has_prev_frame = true;
        return err;
    }

    int min_x = s_display_width;
    int min_y = s_display_height;
    int max_x = -1;
    int max_y = -1;
    size_t bytes_per_row = (size_t)(s_display_width + 1) / 2;

    for (int y = 0; y < s_display_height; y++) {
        size_t row_offset = (size_t)y * bytes_per_row;
        for (size_t x_byte = 0; x_byte < bytes_per_row; x_byte++) {
            uint8_t new_value = buffer[row_offset + x_byte];
            uint8_t old_value = s_prev_buffer[row_offset + x_byte];
            if (new_value == old_value) {
                continue;
            }

            int x = (int)(x_byte * 2);
            if ((new_value & 0xF0) != (old_value & 0xF0)) {
                update_dirty_bounds(x, y, &min_x, &min_y, &max_x, &max_y);
            }
            if (x + 1 < s_display_width && (new_value & 0x0F) != (old_value & 0x0F)) {
                update_dirty_bounds(x + 1, y, &min_x, &min_y, &max_x, &max_y);
            }
        }
    }

    if (max_x < min_x || max_y < min_y) {
        return 0;
    }

    int aligned_min_x = min_x & ~1;
    int aligned_max_x = max_x | 1;
    if (aligned_max_x >= s_display_width) {
        aligned_max_x = s_display_width - 1;
    }

    int width = aligned_max_x - aligned_min_x + 1;
    int height = max_y - min_y + 1;
    int logical_min_x = s_logical_width;
    int logical_min_y = s_logical_height;
    int logical_max_x = -1;
    int logical_max_y = -1;

    int corners_x[4] = { aligned_min_x, aligned_max_x, aligned_min_x, aligned_max_x };
    int corners_y[4] = { min_y, min_y, max_y, max_y };
    for (int i = 0; i < 4; i++) {
        int mapped_x = 0;
        int mapped_y = 0;
        map_physical_to_logical(corners_x[i], corners_y[i], &mapped_x, &mapped_y);
        update_dirty_bounds(mapped_x, mapped_y, &logical_min_x, &logical_min_y, &logical_max_x, &logical_max_y);
    }

    if (logical_max_x < logical_min_x || logical_max_y < logical_min_y) {
        return 0;
    }

    int logical_width = logical_max_x - logical_min_x + 1;
    int logical_height = logical_max_y - logical_min_y + 1;
    EpdRect area = {
        .x = logical_min_x,
        .y = logical_min_y,
        .width = logical_width,
        .height = logical_height,
    };

    size_t start_byte = (size_t)aligned_min_x / 2;
    size_t byte_count = (size_t)(width + 1) / 2;
    for (int y = 0; y < height; y++) {
        size_t row = (size_t)(min_y + y) * bytes_per_row + start_byte;
        memcpy(
            s_framebuffer + row,
            buffer + row,
            byte_count
        );
    }

    epd_poweron();
    int temperature = (int)epd_ambient_temperature();
    int err = check_draw_error(epd_hl_update_area(&s_highlevel, MODE_GC16, temperature, area));
    epd_poweroff();
    memcpy(s_prev_buffer, buffer, s_buffer_size);
    return err;
}

static void map_physical_to_logical(int phys_x, int phys_y, int* out_x, int* out_y) {
    int x = 0;
    int y = 0;

    switch (s_rotation) {
        case EPD_ROT_LANDSCAPE:
            x = phys_y;
            y = s_display_width - 1 - phys_x;
            break;
        case EPD_ROT_INVERTED_PORTRAIT:
            x = s_display_width - 1 - phys_x;
            y = s_display_height - 1 - phys_y;
            break;
        case EPD_ROT_INVERTED_LANDSCAPE:
            x = s_display_height - 1 - phys_y;
            y = phys_x;
            break;
        case EPD_ROT_PORTRAIT:
        default:
            x = phys_x;
            y = phys_y;
            break;
    }

    if (x < 0) {
        x = 0;
    } else if (x >= s_logical_width) {
        x = s_logical_width - 1;
    }

    if (y < 0) {
        y = 0;
    } else if (y >= s_logical_height) {
        y = s_logical_height - 1;
    }

    *out_x = x;
    *out_y = y;
}
