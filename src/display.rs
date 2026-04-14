use alloc::ffi::CString;
use core::ffi::c_char;

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
