
use display::SnFramebufferDisplay;

pub mod display;
pub mod writer;

pub fn init() -> Option<SnFramebufferDisplay> {
    if let Some(framebuffer_response) = crate::limine::FRAMEBUFFER_REQUEST.get_response() {
        if let Some(mut framebuffer) = framebuffer_response.framebuffers().next() {
            let display = SnFramebufferDisplay::new(&mut framebuffer);

            return Some(display);
        }
    }

    None
}