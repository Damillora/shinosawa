use display::SnFramebufferDisplay;
use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb888,
    text::{Alignment, LineHeight, Text, TextStyleBuilder},
};
use limine::request::FramebufferRequest;
use profont::PROFONT_18_POINT;

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

pub mod display;

pub fn init() {
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(mut framebuffer) = framebuffer_response.framebuffers().next() {
            use embedded_graphics::prelude::*;
            let height = framebuffer.height() as i32;
            let width = framebuffer.width() as i32;
            let mut display = SnFramebufferDisplay::new(&mut framebuffer);

            for i in 0..height {
                for j in 0..width {
                    display.draw_pixel(Pixel(Point { x: j, y: i }, Rgb888::new(0x00, 0xaf, 0xcc)));
                }
            }

            let character_style =
                MonoTextStyle::new(&PROFONT_18_POINT, Rgb888::new(0xFF, 0xFF, 0xFF));

            Text::new("shinosawa hiro", Point::new(5, 18), character_style)
                .draw(&mut display)
                .unwrap();
        }
    }
}
