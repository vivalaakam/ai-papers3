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

#include <esp_log.h>
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
    if (!s_initialized) {
        int init_result = papers3_display_init();
        if (init_result != 0) {
            return init_result;
        }
    }

    uint8_t* framebuffer = epd_hl_get_framebuffer(&s_highlevel);
    epd_hl_set_all_white(&s_highlevel);

    EpdRect rect = {
        .x = x,
        .y = y,
        .width = width,
        .height = height,
    };

    epd_fill_rect(rect, 0xD0, framebuffer);
    epd_draw_rect(rect, 0x00, framebuffer);

    draw_text(framebuffer, x + 16, y + 16, text, 0x00);
    draw_bitmap(
        framebuffer,
        bitmap,
        bitmap_width,
        bitmap_height,
        bitmap_x,
        bitmap_y
    );

    epd_poweron();
    int temperature = (int)epd_ambient_temperature();
    int err = check_draw_error(epd_hl_update_screen(&s_highlevel, MODE_GC16, temperature));
    epd_poweroff();

    return err;
}

void papers3_display_deinit(void) {
    if (!s_initialized) {
        return;
    }

    epd_deinit();
    s_initialized = false;
}
