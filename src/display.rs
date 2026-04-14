use alloc::ffi::CString;
use core::ffi::c_char;

use crate::BmpImage;

unsafe extern "C" {
    fn papers3_display_init() -> i32;
    fn papers3_display_set_rotation(rotation_degrees: i32) -> i32;
    fn papers3_display_render_scene(
        text: *const c_char,
        bitmap: *const u8,
        bitmap_width: i32,
        bitmap_height: i32,
        bitmap_x: i32,
        bitmap_y: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> i32;
}

pub fn set_display_rotation(rotation_degrees: i32) -> Result<(), i32> {
    let result = unsafe { papers3_display_set_rotation(rotation_degrees) };
    if result == 0 {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn show_scene(text: &str, image: Option<&BmpImage>) -> Result<(), i32> {
    let text = CString::new(text).map_err(|_| -1)?;

    let init_result = unsafe { papers3_display_init() };
    if init_result != 0 {
        return Err(init_result);
    }

    let (bitmap_ptr, bitmap_width, bitmap_height) = match image {
        Some(image) => (
            image.pixels.as_ptr(),
            image.width as i32,
            image.height as i32,
        ),
        None => (core::ptr::null(), 0, 0),
    };

    let draw_result = unsafe {
        papers3_display_render_scene(
            text.as_ptr(),
            bitmap_ptr,
            bitmap_width,
            bitmap_height,
            40,
            250,
            40,
            40,
            460,
            180,
        )
    };

    if draw_result == 0 {
        Ok(())
    } else {
        Err(draw_result)
    }
}
