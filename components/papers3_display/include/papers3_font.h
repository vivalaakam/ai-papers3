#pragma once

#include <stdint.h>

typedef struct {
    uint16_t codepoint;
    uint16_t width;
    uint16_t height;
    int16_t x_offset;
    int16_t y_offset;
    uint16_t x_advance;
    uint32_t bitmap_offset;
} FontGlyph;

typedef struct {
    uint16_t size;
    int16_t ascent;
    int16_t descent;
    uint16_t line_height;
    uint32_t glyph_count;
    const FontGlyph* glyphs;
    const uint8_t* bitmap;
} FontFace;
