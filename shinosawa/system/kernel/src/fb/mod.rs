use core::fmt::Write;

use display::SnFramebufferDisplay;
use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb888,
    text::Text,
};
use limine::request::FramebufferRequest;
use profont::PROFONT_18_POINT;
use writer::SnFramebufferWriter;

use crate::log::{SnLogger, LOGGER};

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

pub mod display;
pub mod writer;

pub fn init() {
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(mut framebuffer) = framebuffer_response.framebuffers().next() {
            let mut display = SnFramebufferDisplay::new(&mut framebuffer);

            let mut writer = SnFramebufferWriter::new(display);
            writer.clear();

            let logger = LOGGER.get_or_init(move || SnLogger::new(writer));
        }
    }
}
