use core::slice;

use embedded_graphics::{pixelcolor::Rgb888, prelude::*};
use limine::framebuffer::Framebuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

pub struct SnFramebufferDisplay {
    buffer: &'static mut [u8],
    bpp: u16,
    pitch: usize,
    pub width: usize,
    pub height: usize,
}
impl SnFramebufferDisplay {
    pub fn new(framebuffer: &mut Framebuffer) -> SnFramebufferDisplay {
        let fb_size = (framebuffer.height() * framebuffer.pitch()) as usize;

        let buffer_slice = unsafe { slice::from_raw_parts_mut(framebuffer.addr(), fb_size) };

        SnFramebufferDisplay {
            buffer: buffer_slice,
            pitch: framebuffer.pitch() as usize,
            bpp: framebuffer.bpp(),
            height: framebuffer.height() as usize,
            width: framebuffer.width() as usize,
        }
    }

    fn set_pixel_in(&mut self, position: Position, color: Color) {
        let pixel_offset = position.y * self.pitch as usize + position.x * (self.bpp / 8) as usize;

        let pixel_buffer = &mut self.buffer[pixel_offset..];

        pixel_buffer[3] = color.alpha;
        pixel_buffer[2] = color.red;
        pixel_buffer[1] = color.green;
        pixel_buffer[0] = color.blue;
    }

    pub fn draw_pixel(&mut self, Pixel(coordinates, color): Pixel<Rgb888>) {
        // ignore any out of bounds pixels
        let (width, height) = { (self.width as usize, self.height as usize) };

        let (x, y) = {
            let c: (i32, i32) = coordinates.into();
            (c.0 as usize, c.1 as usize)
        };

        if (0..width).contains(&x) && (0..height).contains(&y) {
            let color = Color {
                red: color.r(),
                green: color.g(),
                blue: color.b(),
                alpha: 0xFF,
            };

            self.set_pixel_in(Position { x, y }, color);
        }
    }
}

impl DrawTarget for SnFramebufferDisplay {
    type Color = Rgb888;

    /// Drawing operations can never fail.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for pixel in pixels.into_iter() {
            self.draw_pixel(pixel);
        }

        Ok(())
    }
}

impl OriginDimensions for SnFramebufferDisplay {
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}
