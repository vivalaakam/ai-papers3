use embedded_graphics::geometry::Size;
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator::{
    OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};

use crate::display::{DisplayError, DisplayTarget};
use crate::{BmpImage, TouchPoint};

pub struct SimulatedDisplay {
    display: SimulatorDisplay<Gray4>,
    window: Window,
    logical_width: i32,
    logical_height: i32,
    physical_width: i32,
    physical_height: i32,
    rotation: i32,
    pending_events: Vec<TouchPoint>,
    quit_requested: bool,
}

impl SimulatedDisplay {
    pub fn new(width: i32, height: i32, scale: u32, rotation_degrees: i32) -> Self {
        let rotation = ((rotation_degrees / 90) % 4 + 4) % 4;
        let (physical_width, physical_height) = if rotation % 2 == 0 {
            (width, height)
        } else {
            (height, width)
        };
        let display =
            SimulatorDisplay::new(Size::new(physical_width as u32, physical_height as u32));
        let output_settings = OutputSettingsBuilder::new().scale(scale).build();
        let window = Window::new("ai-papers3 simulator", &output_settings);
        Self {
            display,
            window,
            logical_width: width,
            logical_height: height,
            physical_width,
            physical_height,
            rotation,
            pending_events: Vec::new(),
            quit_requested: false,
        }
    }

    fn draw_bitmap_internal(&mut self, image: &BmpImage, x: i32, y: i32) {
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
                self.draw_pixel(target_x, target_y, pixel);
            }
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

    fn pump_events(&mut self) {
        for event in self.window.events() {
            match event {
                SimulatorEvent::Quit => {
                    self.quit_requested = true;
                }
                SimulatorEvent::MouseButtonUp { point, .. } => {
                    if point.x >= 0 && point.y >= 0 {
                        self.pending_events.push(TouchPoint {
                            x: point.x as u16,
                            y: point.y as u16,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

impl DisplayTarget for SimulatedDisplay {
    fn width(&self) -> i32 {
        self.logical_width
    }

    fn height(&self) -> i32 {
        self.logical_height
    }

    fn clear(&mut self, color: Gray4) {
        let _ = self.display.clear(color);
    }

    fn flush(&mut self) -> Result<(), DisplayError> {
        self.window.update(&self.display);
        Ok(())
    }

    fn draw_pixel(&mut self, x: i32, y: i32, nibble: u8) {
        let Some((phys_x, phys_y)) = self.map_point(x, y) else {
            return;
        };
        let color = Gray4::new(nibble & 0x0F);
        let _ = self
            .display
            .draw_iter(core::iter::once(Pixel(Point::new(phys_x, phys_y), color)));
    }

    fn draw_bitmap(&mut self, image: &BmpImage, x: i32, y: i32) {
        self.draw_bitmap_internal(image, x, y);
    }

    fn poll_events(&mut self) -> bool {
        self.pump_events();
        self.quit_requested
    }

    fn drain_input(&mut self) -> Vec<TouchPoint> {
        self.pump_events();
        self.pending_events.drain(..).collect()
    }
}
