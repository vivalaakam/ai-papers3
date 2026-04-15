use alloc::ffi::CString;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::ffi::c_char;

use embedded_graphics::geometry::Size;
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::*;
use log::info;
use crate::BmpImage;

unsafe extern "C" {
    fn papers3_display_set_rotation(rotation_degrees: i32) -> i32;
    fn papers3_display_set_font_size(font_size: i32) -> i32;
    fn papers3_display_begin() -> i32;
    fn papers3_display_draw_text(text: *const c_char, x: i32, y: i32) -> i32;
    fn papers3_display_draw_bitmap(
        bitmap: *const u8,
        bitmap_width: i32,
        bitmap_height: i32,
        bitmap_x: i32,
        bitmap_y: i32,
    ) -> i32;
    fn papers3_display_draw_rect(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        fill_color: i32,
        stroke_color: i32,
    ) -> i32;
    fn papers3_display_commit() -> i32;
    fn papers3_display_width() -> i32;
    fn papers3_display_height() -> i32;
    fn papers3_display_physical_width() -> i32;
    fn papers3_display_physical_height() -> i32;
    fn papers3_display_get_rotation() -> i32;
    fn papers3_display_present(buffer: *const u8, width: i32, height: i32) -> i32;
}

pub struct EmbeddedDisplay {
    logical_width: i32,
    logical_height: i32,
    physical_width: i32,
    physical_height: i32,
    rotation: i32,
    buffer: Vec<u8>,
}

impl EmbeddedDisplay {
    pub fn new() -> Result<Self, i32> {
        display_begin()?;
        let logical_width = unsafe { papers3_display_width() };
        let logical_height = unsafe { papers3_display_height() };
        let physical_width = unsafe { papers3_display_physical_width() };
        let physical_height = unsafe { papers3_display_physical_height() };
        let rotation = unsafe { papers3_display_get_rotation() };
        info!("screen: x1:{} y1:{} x2:{} y2:{}", physical_width, physical_height, logical_width, logical_height);
        if logical_width <= 0 || logical_height <= 0 || physical_width <= 0 || physical_height <= 0 {
            return Err(-1);
        }
        let buffer_size = ((physical_width + 1) / 2 * physical_height) as usize;
        let mut buffer = vec![0xFF; buffer_size];
        Self::fill_buffer(&mut buffer, Gray4::WHITE);
        Ok(Self {
            logical_width,
            logical_height,
            physical_width,
            physical_height,
            rotation,
            buffer,
        })
    }

    pub fn width(&self) -> i32 {
        self.logical_width
    }

    pub fn height(&self) -> i32 {
        self.logical_height
    }

    pub fn clear(&mut self, color: Gray4) {
        Self::fill_buffer(&mut self.buffer, color);
    }

    pub fn scaled(&mut self, scale: i32) -> ScaledDisplay<'_> {
        ScaledDisplay::new(self, scale)
    }

    pub fn flush(&mut self) -> Result<(), i32> {
        let result = unsafe {
            papers3_display_present(
                self.buffer.as_ptr(),
                self.physical_width,
                self.physical_height,
            )
        };
        if result == 0 { Ok(()) } else { Err(result) }
    }

    pub fn draw_bitmap(&mut self, image: &BmpImage, x: i32, y: i32) {
        let width = self.logical_width;
        let height = self.logical_height;
        for row in 0..image.height {
            let target_y = y + row as i32;
            if target_y < 0 || target_y >= height {
                continue;
            }
            let row_start = row * image.width;
            for col in 0..image.width {
                let target_x = x + col as i32;
                if target_x < 0 || target_x >= width {
                    continue;
                }
                let pixel = image.pixels[row_start + col] >> 4;
                self.set_pixel_nibble(target_x, target_y, pixel);
            }
        }
    }

    fn fill_buffer(buffer: &mut [u8], color: Gray4) {
        let nibble = color.luma() & 0x0F;
        let byte = (nibble << 4) | nibble;
        buffer.fill(byte);
    }

    /// Установить один пиксель (nibble 0-15) в логических координатах.
    pub(crate) fn draw_pixel(&mut self, x: i32, y: i32, nibble: u8) {
        self.set_pixel_nibble(x, y, nibble);
    }

    fn set_pixel_nibble(&mut self, x: i32, y: i32, nibble: u8) {
        let Some((phys_x, phys_y)) = self.map_point(x, y) else {
            return;
        };
        let nibble = nibble & 0x0F;
        let bytes_per_row = ((self.physical_width + 1) / 2) as usize;
        let index = phys_y as usize * bytes_per_row + (phys_x as usize / 2);
        let byte = &mut self.buffer[index];
        if phys_x % 2 == 0 {
            *byte = (*byte & 0x0F) | (nibble << 4);
        } else {
            *byte = (*byte & 0xF0) | nibble;
        }
    }

    fn map_point(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        if x < 0 || y < 0 || x >= self.logical_width || y >= self.logical_height {
            return None;
        }

        let w = self.physical_width;
        let h = self.physical_height;
        let (phys_x, phys_y) = match self.rotation {
            0 => (x, y),
            1 => (w - 1 - y, x),
            2 => (w - 1 - x, h - 1 - y),
            3 => (y, h - 1 - x),
            _ => (x, y),
        };

        if phys_x < 0 || phys_y < 0 || phys_x >= w || phys_y >= h {
            return None;
        }
        Some((phys_x, phys_y))
    }
}

impl DrawTarget for EmbeddedDisplay {
    type Color = Gray4;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            self.set_pixel_nibble(point.x, point.y, color.luma());
        }
        Ok(())
    }
}

impl OriginDimensions for EmbeddedDisplay {
    fn size(&self) -> Size {
        Size::new(self.logical_width as u32, self.logical_height as u32)
    }
}

pub struct ScaledDisplay<'a> {
    inner: &'a mut EmbeddedDisplay,
    scale: i32,
}

impl<'a> ScaledDisplay<'a> {
    pub fn new(inner: &'a mut EmbeddedDisplay, scale: i32) -> Self {
        let scale = if scale < 1 { 1 } else { scale };
        Self { inner, scale }
    }
}

impl DrawTarget for ScaledDisplay<'_> {
    type Color = Gray4;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let scale = self.scale;
        for Pixel(point, color) in pixels {
            let base_x = point.x * scale;
            let base_y = point.y * scale;
            for dy in 0..scale {
                for dx in 0..scale {
                    self.inner
                        .set_pixel_nibble(base_x + dx, base_y + dy, color.luma());
                }
            }
        }
        Ok(())
    }
}

impl OriginDimensions for ScaledDisplay<'_> {
    fn size(&self) -> Size {
        let width = (self.inner.logical_width / self.scale).max(1) as u32;
        let height = (self.inner.logical_height / self.scale).max(1) as u32;
        Size::new(width, height)
    }
}

pub fn set_display_font_size(font_size: i32) -> Result<(), i32> {
    let result = unsafe { papers3_display_set_font_size(font_size) };
    if result == 0 { Ok(()) } else { Err(result) }
}

pub fn set_display_rotation(rotation_degrees: i32) -> Result<(), i32> {
    let result = unsafe { papers3_display_set_rotation(rotation_degrees) };
    if result == 0 { Ok(()) } else { Err(result) }
}

pub fn display_begin() -> Result<(), i32> {
    let result = unsafe { papers3_display_begin() };
    if result == 0 { Ok(()) } else { Err(result) }
}

pub fn display_draw_text(text: &str, x: i32, y: i32) -> Result<(), i32> {
    let text = CString::new(text).map_err(|_| -1)?;
    let result = unsafe { papers3_display_draw_text(text.as_ptr(), x, y) };
    if result == 0 { Ok(()) } else { Err(result) }
}

pub fn display_draw_bitmap(image: &BmpImage, x: i32, y: i32) -> Result<(), i32> {
    let result = unsafe {
        papers3_display_draw_bitmap(
            image.pixels.as_ptr(),
            image.width as i32,
            image.height as i32,
            x,
            y,
        )
    };
    if result == 0 { Ok(()) } else { Err(result) }
}

pub fn display_draw_rect(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    fill_color: Option<u8>,
    stroke_color: Option<u8>,
) -> Result<(), i32> {
    let fill = fill_color.map_or(-1, |value| value as i32);
    let stroke = stroke_color.map_or(-1, |value| value as i32);
    let result = unsafe { papers3_display_draw_rect(x, y, width, height, fill, stroke) };
    if result == 0 { Ok(()) } else { Err(result) }
}

pub fn display_commit() -> Result<(), i32> {
    let result = unsafe { papers3_display_commit() };
    if result == 0 { Ok(()) } else { Err(result) }
}
