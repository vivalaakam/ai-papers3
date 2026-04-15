mod ubuntu_regular_13;
mod ubuntu_regular_18;
mod ubuntu_regular_24;

pub use ubuntu_regular_13::UBUNTU_REGULAR_13;
pub use ubuntu_regular_18::UBUNTU_REGULAR_18;
pub use ubuntu_regular_24::UBUNTU_REGULAR_24;

// ─── GlyphInfo ────────────────────────────────────────────────────────────────

/// Метрики и позиция битмапа одного глифа.
#[derive(Clone, Copy)]
pub struct GlyphInfo {
    /// Unicode codepoint.
    pub codepoint: u32,
    /// Ширина и высота битмапа (в пикселях).
    pub width: u8,
    pub height: u8,
    /// Смещение от позиции курсора до левого края битмапа.
    pub x_offset: i16,
    /// Смещение от базовой линии до верхнего края битмапа (обычно ≤ 0).
    pub y_offset: i16,
    /// Горизонтальный шаг курсора после этого символа.
    pub x_advance: u8,
    /// Байтовый offset в массиве bitmap.
    pub bitmap_offset: u32,
}

// ─── FontFace ─────────────────────────────────────────────────────────────────

/// Пропорциональный bitmap-шрифт с поддержкой Unicode.
pub struct FontFace<'a> {
    /// Размер шрифта в пикселях (приблизительная высота заглавной буквы).
    pub size: u8,
    /// Расстояние от верхнего края строки до базовой линии.
    pub ascent: i16,
    /// Расстояние от базовой линии до нижнего края строки (положительное).
    pub descent: i16,
    /// Полная высота строки (ascent + descent + межстрочный интервал).
    pub line_height: i16,
    /// Таблица глифов, отсортированная по codepoint.
    pub glyphs: &'a [GlyphInfo],
    /// Сжатые битмапы всех глифов (MSB-first, 1 бит = 1 пиксель).
    pub bitmap: &'a [u8],
}

impl<'a> FontFace<'a> {
    /// Найти глиф по символу. O(log N) — бинарный поиск по codepoint.
    pub fn find_glyph(&self, c: char) -> Option<&GlyphInfo> {
        let cp = c as u32;
        self.glyphs
            .binary_search_by_key(&cp, |g| g.codepoint)
            .ok()
            .map(|i| &self.glyphs[i])
    }

    /// Ширина строки текста в пикселях (сумма x_advance всех символов).
    pub fn measure_line(&self, text: &str) -> i32 {
        text.chars()
            .map(|c| {
                self.find_glyph(c)
                    .map(|g| g.x_advance as i32)
                    .unwrap_or(self.size as i32)
            })
            .sum()
    }

    /// Размер многострочного текста → (max_width, total_height).
    pub fn measure_text(&self, text: &str) -> (i32, i32) {
        let mut max_w = 0i32;
        let mut line_count = 0i32;
        for line in text.split('\n') {
            max_w = max_w.max(self.measure_line(line));
            line_count += 1;
        }
        (max_w, line_count.max(1) * self.line_height as i32)
    }
}
