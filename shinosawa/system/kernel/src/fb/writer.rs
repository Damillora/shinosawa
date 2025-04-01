use core::fmt;

use embedded_graphics::{
    mono_font::MonoTextStyle, pixelcolor::Rgb888, prelude::Point, text::Text, Drawable, Pixel
};
use profont::PROFONT_18_POINT;

use super::display::SnFramebufferDisplay;

const LINE_SPACING: usize = 5;
const BORDER_PADDING: usize = 5;

pub struct SnFramebufferWriter {
    display: SnFramebufferDisplay,
    x_pos: usize,
    y_pos: usize,
    x_char: usize,
    y_char: usize,
}

impl SnFramebufferWriter {
    pub fn new(display: SnFramebufferDisplay) -> SnFramebufferWriter {
        let logger = Self {
            display: display,
            x_pos: BORDER_PADDING,
            y_pos: 18 + BORDER_PADDING,
            x_char: PROFONT_18_POINT.character_size.width as usize,
            y_char: PROFONT_18_POINT.character_size.height as usize,
        };

        logger
    }

    fn newline(&mut self) {
        self.y_pos += self.x_char + LINE_SPACING;
        self.carriage_return();
    }

    fn carriage_return(&mut self) {
        self.x_pos = BORDER_PADDING;
    }

    fn width(&self) -> usize {
        self.display.width
    }

    fn height(&self) -> usize {
        self.display.height
    }
    /// Writes a single char to the framebuffer. Takes care of special control characters, such as
    /// newlines and carriage returns.
    fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            '\r' => self.carriage_return(),
            c => {
                let character_style =
                    MonoTextStyle::new(&PROFONT_18_POINT, Rgb888::new(0xFF, 0xFF, 0xFF));
                let str = &mut [0u8; 4];
                let new_str = c.encode_utf8(str);
                let next_pos = Text::new(new_str, Point::new(self.x_pos as i32, self.y_pos as i32), character_style)
                    .draw(&mut self.display)
                    .unwrap();
                self.x_pos += self.x_char;
                
                if self.x_pos > self.display.width {
                    self.newline();
                    self.carriage_return();
                }
            }
        }
    }

    pub fn clear(&mut self) {
        for i in 0..self.display.height {
            for j in 0..self.display.width {
                self.display.draw_pixel(Pixel(Point { x: j as i32, y: i as i32}, Rgb888::new(0x00, 0xaf, 0xcc)));
            }
        }

    }
}

unsafe impl Send for SnFramebufferWriter {}
unsafe impl Sync for SnFramebufferWriter {}

impl fmt::Write for SnFramebufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }
        
        Ok(())
    }
}
