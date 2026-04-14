#!/usr/bin/env python3
import argparse
import math
import os
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


def build_codepoints():
    codepoints = list(range(0x20, 0x7F))
    codepoints += list(range(0x410, 0x450))
    codepoints += [0x401, 0x451]
    return sorted(set(codepoints))


def render_glyph(font, codepoint, ascent, descent):
    char = chr(codepoint)
    canvas_width = font.getlength(char) + font.size * 2
    canvas_height = ascent + descent + font.size
    canvas = Image.new("1", (int(canvas_width), int(canvas_height)), 0)
    draw = ImageDraw.Draw(canvas)

    try:
        draw.text((0, ascent), char, font=font, fill=1, anchor="ls")
    except TypeError:
        draw.text((0, 0), char, font=font, fill=1)

    bbox = canvas.getbbox()
    if bbox is None:
        x_advance = int(round(font.getlength(char)))
        return {
            "codepoint": codepoint,
            "width": 0,
            "height": 0,
            "x_offset": 0,
            "y_offset": 0,
            "x_advance": x_advance,
            "bitmap": [],
        }

    glyph_image = canvas.crop(bbox)
    width, height = glyph_image.size
    pixels = glyph_image.load()

    bytes_per_row = (width + 7) // 8
    bitmap = []
    for row in range(height):
        row_bytes = [0] * bytes_per_row
        for col in range(width):
            if pixels[col, row]:
                row_bytes[col // 8] |= 0x80 >> (col % 8)
        bitmap.extend(row_bytes)

    x_advance = int(round(font.getlength(char)))
    x_offset = bbox[0]
    y_offset = bbox[1] - ascent

    return {
        "codepoint": codepoint,
        "width": width,
        "height": height,
        "x_offset": x_offset,
        "y_offset": y_offset,
        "x_advance": x_advance,
        "bitmap": bitmap,
    }


def generate_font(ttf_path, size, output_path, name):
    font = ImageFont.truetype(ttf_path, size)
    ascent, descent = font.getmetrics()
    codepoints = build_codepoints()

    glyphs = []
    bitmap_data = []
    for codepoint in codepoints:
        glyph = render_glyph(font, codepoint, ascent, descent)
        if glyph is None:
            continue
        glyph["bitmap_offset"] = len(bitmap_data)
        bitmap_data.extend(glyph["bitmap"])
        glyphs.append(glyph)

    line_height = ascent + descent + math.ceil(size * 0.25)

    with open(output_path, "w", encoding="utf-8") as output:
        output.write("#pragma once\n\n")
        output.write('#include "papers3_font.h"\n\n')
        output.write(f"static const uint8_t {name}_bitmap[] = {{\n")
        for idx, byte in enumerate(bitmap_data):
            if idx % 12 == 0:
                output.write("    ")
            output.write(f"0x{byte:02X}, ")
            if idx % 12 == 11:
                output.write("\n")
        if len(bitmap_data) % 12 != 0:
            output.write("\n")
        output.write("};\n\n")

        output.write(f"static const FontGlyph {name}_glyphs[] = {{\n")
        for glyph in glyphs:
            output.write(
                "    { "
                f"0x{glyph['codepoint']:04X}, "
                f"{glyph['width']}, {glyph['height']}, "
                f"{glyph['x_offset']}, {glyph['y_offset']}, "
                f"{glyph['x_advance']}, {glyph['bitmap_offset']} "
                "},\n"
            )
        output.write("};\n\n")

        output.write(
            f"static const FontFace {name}_font = {{\n"
            f"    {size},\n"
            f"    {ascent},\n"
            f"    {descent},\n"
            f"    {line_height},\n"
            f"    {len(glyphs)},\n"
            f"    {name}_glyphs,\n"
            f"    {name}_bitmap,\n"
            "};\n"
        )


def main():
    parser = argparse.ArgumentParser(description="Generate C font header from TTF.")
    parser.add_argument("ttf_path", help="Path to TTF font file.")
    parser.add_argument("size", type=int, help="Font size in points.")
    parser.add_argument("output_path", help="Path to output header file.")
    parser.add_argument("name", help="Base name for generated symbols.")
    args = parser.parse_args()

    if not Path(args.ttf_path).exists():
        raise SystemExit(f"TTF not found: {args.ttf_path}")

    os.makedirs(os.path.dirname(args.output_path), exist_ok=True)
    generate_font(args.ttf_path, args.size, args.output_path, args.name)


if __name__ == "__main__":
    main()
