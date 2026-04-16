use crate::display::DisplayTarget;
use crate::fonts::{FontFace, GlyphInfo, UBUNTU_REGULAR_13, UBUNTU_REGULAR_18, UBUNTU_REGULAR_24};
use crate::image::BmpImage;

// ─── Color ───────────────────────────────────────────────────────────────────

/// Оттенок серого: 0 = чёрный, 15 = белый.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Color(pub u8);

impl Color {
    pub const BLACK: Color = Color(0);
    pub const WHITE: Color = Color(15);
    pub const DARK_GRAY: Color = Color(4);
    pub const GRAY: Color = Color(8);
    pub const LIGHT_GRAY: Color = Color(11);
}

// ─── UiRect ──────────────────────────────────────────────────────────────────

/// Прямоугольник в пикселях. x, y — верхний левый угол.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl UiRect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Проверить, попадает ли точка (px, py) внутрь прямоугольника.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

// ─── FontSize ─────────────────────────────────────────────────────────────────

/// Выбор размера шрифта Ubuntu Regular с поддержкой Unicode/Кириллицы.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum FontSize {
    Small, // Ubuntu Regular 13 px
    #[default]
    Medium, // Ubuntu Regular 18 px
    Large, // Ubuntu Regular 24 px
}

impl FontSize {
    pub fn font(self) -> &'static FontFace<'static> {
        match self {
            FontSize::Small => &UBUNTU_REGULAR_13,
            FontSize::Medium => &UBUNTU_REGULAR_18,
            FontSize::Large => &UBUNTU_REGULAR_24,
        }
    }

    /// Высота одной строки в пикселях.
    pub fn line_height(self) -> i32 {
        self.font().line_height as i32
    }
}

// ─── TextStyle ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default)]
pub struct TextStyle {
    pub font_size: FontSize,
    pub color: Color,
}

// ─── DrawContext ──────────────────────────────────────────────────────────────

/// Контекст рисования. Компоненты получают его в методе `draw()`.
pub struct DrawContext<'a> {
    display: &'a mut dyn DisplayTarget,
}

impl<'a> DrawContext<'a> {
    pub fn new(display: &'a mut dyn DisplayTarget) -> Self {
        Self { display }
    }

    /// Залить прямоугольник цветом.
    pub fn fill_rect(&mut self, rect: UiRect, color: Color) {
        if rect.width <= 0 || rect.height <= 0 {
            return;
        }
        let end_x = rect.x + rect.width;
        let end_y = rect.y + rect.height;
        for y in rect.y..end_y {
            for x in rect.x..end_x {
                self.display.draw_pixel(x, y, color.0 & 0x0F);
            }
        }
    }

    /// Нарисовать рамку вокруг прямоугольника (без заливки).
    pub fn stroke_rect(&mut self, rect: UiRect, color: Color, thickness: u32) {
        if rect.width <= 0 || rect.height <= 0 || thickness == 0 {
            return;
        }
        let thickness = thickness as i32;
        let end_x = rect.x + rect.width;
        let end_y = rect.y + rect.height;
        for offset in 0..thickness {
            let top = rect.y + offset;
            let bottom = end_y - 1 - offset;
            for x in rect.x..end_x {
                self.display.draw_pixel(x, top, color.0 & 0x0F);
                self.display.draw_pixel(x, bottom, color.0 & 0x0F);
            }

            let left = rect.x + offset;
            let right = end_x - 1 - offset;
            for y in rect.y..end_y {
                self.display.draw_pixel(left, y, color.0 & 0x0F);
                self.display.draw_pixel(right, y, color.0 & 0x0F);
            }
        }
    }

    /// Нарисовать текст (включая многострочный, разделитель '\n').
    /// `x`, `y` — верхний левый угол первой строки.
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, style: TextStyle) {
        if text.is_empty() {
            return;
        }
        let font = style.font_size.font();
        let nibble = style.color.0 & 0x0F;
        let mut cur_y = y;

        for line in text.split('\n') {
            let baseline_y = cur_y + font.ascent as i32;
            let mut cur_x = x;
            for c in line.chars() {
                match font.find_glyph(c) {
                    Some(glyph) => {
                        Self::draw_glyph(self.display, font, glyph, cur_x, baseline_y, nibble);
                        cur_x += glyph.x_advance as i32;
                    }
                    None => {
                        cur_x += font.size as i32;
                    }
                }
            }
            cur_y += font.line_height as i32;
        }
    }

    /// Нарисовать один глиф из таблицы шрифта.
    fn draw_glyph(
        display: &mut dyn DisplayTarget,
        font: &FontFace<'_>,
        glyph: &GlyphInfo,
        cursor_x: i32,
        baseline_y: i32,
        nibble: u8,
    ) {
        if glyph.width == 0 || glyph.height == 0 {
            return;
        }
        let glyph_x = cursor_x + glyph.x_offset as i32;
        let glyph_y = baseline_y + glyph.y_offset as i32;
        let w = glyph.width as usize;
        let h = glyph.height as usize;
        let bytes_per_row = w.div_ceil(8);
        let base = glyph.bitmap_offset as usize;

        for row in 0..h {
            for col in 0..w {
                let byte_idx = base + row * bytes_per_row + col / 8;
                if byte_idx >= font.bitmap.len() {
                    break;
                }
                let bit = (font.bitmap[byte_idx] >> (7 - col % 8)) & 1;
                if bit != 0 {
                    display.draw_pixel(glyph_x + col as i32, glyph_y + row as i32, nibble);
                }
            }
        }
    }

    /// Измерить размер текста без рисования → (width, height) в пикселях.
    /// Поддерживает многострочный текст (разделитель '\n').
    pub fn measure_text(text: &str, style: TextStyle) -> (i32, i32) {
        style.font_size.font().measure_text(text)
    }

    /// Нарисовать BmpImage с верхним левым углом в (x, y).
    pub fn draw_image(&mut self, image: &BmpImage, x: i32, y: i32) {
        self.display.draw_bitmap(image, x, y);
    }
}
