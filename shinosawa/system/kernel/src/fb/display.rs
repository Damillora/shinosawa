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

pub struct SnFramebufferDisplay<'a> {
    framebuffer: &'a mut Framebuffer<'a>,
}
impl<'a> SnFramebufferDisplay<'a> {
    pub fn new(framebuffer: &'a mut Framebuffer<'a>) -> SnFramebufferDisplay<'a>{
        SnFramebufferDisplay { framebuffer: framebuffer }
    }

    fn set_pixel_in(&self, position: Position, color: Color) {
        let framebuffer = &self.framebuffer;

        let pixel_offset =
            position.y * framebuffer.pitch() as usize + position.x * (framebuffer.bpp() / 8) as usize;
        let color = [color.alpha, color.red, color.green, color.blue, ];
        let color_value = u32::from_be_bytes(color);

        unsafe {
            framebuffer
                .addr()
                .add(pixel_offset )
                .cast::<u32>()
                // .write(0xFF00AFCC)
                .write(color_value)
        };
    }


    pub fn draw_pixel(&mut self, Pixel(coordinates, color): Pixel<Rgb888>) {
        // ignore any out of bounds pixels
        let (width, height) = {

            (self.framebuffer.width() as usize, self.framebuffer.height() as usize)
        };

        let (x, y) = {
            let c: (i32, i32) = coordinates.into();
            (c.0 as usize, c.1 as usize)
        };

        if (0..width).contains(&x) && (0..height).contains(&y) {
            let color = Color { red: color.r(), green: color.g(), blue: color.b(), alpha: 0xFF };

            self.set_pixel_in(Position { x, y }, color);
        }
    }
}

impl<'f> DrawTarget for SnFramebufferDisplay<'f> {
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

impl<'f> OriginDimensions for SnFramebufferDisplay<'f> {
    fn size(&self) -> Size {
        Size::new(self.framebuffer.width() as u32, self.framebuffer.height() as u32)
    }
}