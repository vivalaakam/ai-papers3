use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X13, FONT_9X18};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{Baseline, Text};

use crate::display::EmbeddedDisplay;
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

    pub(crate) fn to_gray4(self) -> Gray4 {
        Gray4::new(self.0 & 0x0F)
    }
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
        Self { x, y, width, height }
    }

    /// Проверить, попадает ли точка (px, py) внутрь прямоугольника.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width
            && py >= self.y
            && py < self.y + self.height
    }
}

// ─── FontSize / TextStyle ─────────────────────────────────────────────────────

/// Выбор размера из встроенных bitmap-шрифтов embedded-graphics.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum FontSize {
    Small,          // 6×13 px на символ
    #[default]
    Medium,         // 9×18 px на символ
    Large,          // 10×20 px на символ
}

impl FontSize {
    pub fn font(self) -> &'static MonoFont<'static> {
        match self {
            FontSize::Small  => &FONT_6X13,
            FontSize::Medium => &FONT_9X18,
            FontSize::Large  => &FONT_10X20,
        }
    }

    pub fn char_width(self) -> i32 {
        self.font().character_size.width as i32
    }

    pub fn line_height(self) -> i32 {
        self.font().character_size.height as i32
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TextStyle {
    pub font_size: FontSize,
    pub color: Color,
}

// ─── DrawContext ──────────────────────────────────────────────────────────────

/// Контекст рисования. Компоненты получают его в методе `draw()`.
pub struct DrawContext<'a> {
    display: &'a mut EmbeddedDisplay,
}

impl<'a> DrawContext<'a> {
    pub fn new(display: &'a mut EmbeddedDisplay) -> Self {
        Self { display }
    }

    /// Залить прямоугольник цветом.
    pub fn fill_rect(&mut self, rect: UiRect, color: Color) {
        if rect.width <= 0 || rect.height <= 0 {
            return;
        }
        let style = PrimitiveStyleBuilder::new()
            .fill_color(color.to_gray4())
            .build();
        Rectangle::new(
            Point::new(rect.x, rect.y),
            Size::new(rect.width as u32, rect.height as u32),
        )
        .into_styled(style)
        .draw(self.display)
        .ok();
    }

    /// Нарисовать рамку вокруг прямоугольника (без заливки).
    pub fn stroke_rect(&mut self, rect: UiRect, color: Color, thickness: u32) {
        if rect.width <= 0 || rect.height <= 0 || thickness == 0 {
            return;
        }
        let style = PrimitiveStyleBuilder::new()
            .stroke_color(color.to_gray4())
            .stroke_width(thickness)
            .build();
        Rectangle::new(
            Point::new(rect.x, rect.y),
            Size::new(rect.width as u32, rect.height as u32),
        )
        .into_styled(style)
        .draw(self.display)
        .ok();
    }

    /// Нарисовать однострочный текст.
    /// `x`, `y` — верхний левый угол (не baseline).
    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, style: TextStyle) {
        if text.is_empty() {
            return;
        }
        let font = style.font_size.font();
        let text_style = MonoTextStyle::new(font, style.color.to_gray4());
        // embedded-graphics рисует от baseline; сдвигаем y вниз на baseline offset
        let baseline_y = y + font.baseline as i32;
        Text::with_baseline(
            text,
            Point::new(x, baseline_y),
            text_style,
            Baseline::Alphabetic,
        )
        .draw(self.display)
        .ok();
    }

    /// Измерить размер текста без рисования → (width, height) в пикселях.
    /// Поддерживает многострочный текст (разделитель '\n').
    pub fn measure_text(text: &str, style: TextStyle) -> (i32, i32) {
        let font = style.font_size.font();
        let char_w = font.character_size.width as i32;
        let line_h = font.character_size.height as i32;

        let mut max_w = 0i32;
        let mut lines = 0i32;
        for line in text.split('\n') {
            max_w = max_w.max(line.len() as i32 * char_w);
            lines += 1;
        }
        (max_w, lines.max(1) * line_h)
    }

    /// Нарисовать BmpImage с верхним левым углом в (x, y).
    pub fn draw_image(&mut self, image: &BmpImage, x: i32, y: i32) {
        self.display.draw_bitmap(image, x, y);
    }
}
