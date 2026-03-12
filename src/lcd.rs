/// LCD framebuffer renderer for the AK820 Pro's 128x128 TFT screen.
/// Uses embedded-graphics for text rendering on an RGB565 framebuffer.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::*,
    text::{Alignment, Text},
};

use crate::protocol::{LCD_WIDTH, LCD_HEIGHT, LCD_PIXELS, LCD_DATA_SIZE};
use crate::stats::Stats;

/// A 128x128 RGB565 framebuffer that implements embedded-graphics DrawTarget.
pub struct LcdFramebuffer {
    pixels: Vec<Rgb565>,
}

impl LcdFramebuffer {
    pub fn new() -> Self {
        Self {
            pixels: vec![Rgb565::BLACK; LCD_PIXELS],
        }
    }

    /// Clear the framebuffer to black.
    pub fn clear_black(&mut self) {
        self.pixels.fill(Rgb565::BLACK);
    }

    /// Export the framebuffer as raw RGB565 bytes (little-endian, 32768 bytes).
    pub fn as_rgb565_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(LCD_DATA_SIZE);
        for pixel in &self.pixels {
            let raw = RawU16::from(*pixel).into_inner();
            data.extend_from_slice(&raw.to_le_bytes());
        }
        data
    }

    /// Render system stats onto the framebuffer.
    /// Shows: current time, CPU temp, GPU temp — all centered, FONT_6X10.
    pub fn render_stats(&mut self, stats: &Stats) {
        self.clear_black();

        // Bright yellow — white is nearly invisible on this TFT
        let style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(31, 63, 0));

        // FONT_6X10: 6px wide, 10px tall. Screen 128x128.
        // y = text baseline. Three rows evenly spaced.

        let now = chrono::Local::now().format("%H:%M:%S").to_string();
        Text::with_alignment(&now, Point::new(64, 35), style, Alignment::Center)
            .draw(self).ok();

        let cpu_str = match stats.cpu_temp_c {
            Some(t) => format!("CPU {:.0}C", t),
            None => "CPU --".to_string(),
        };
        Text::with_alignment(&cpu_str, Point::new(64, 65), style, Alignment::Center)
            .draw(self).ok();

        let gpu_str = match stats.gpu_temp_c {
            Some(t) => format!("GPU {:.0}C", t),
            None => "GPU --".to_string(),
        };
        Text::with_alignment(&gpu_str, Point::new(64, 95), style, Alignment::Center)
            .draw(self).ok();
    }
}

impl DrawTarget for LcdFramebuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.x < LCD_WIDTH as i32
                && coord.y >= 0
                && coord.y < LCD_HEIGHT as i32
            {
                let idx = coord.y as usize * LCD_WIDTH as usize + coord.x as usize;
                self.pixels[idx] = color;
            }
        }
        Ok(())
    }
}

impl OriginDimensions for LcdFramebuffer {
    fn size(&self) -> Size {
        Size::new(LCD_WIDTH, LCD_HEIGHT)
    }
}
