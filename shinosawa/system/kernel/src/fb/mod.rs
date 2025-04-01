
use display::SnFramebufferDisplay;
use limine::request::FramebufferRequest;

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

pub mod display;
pub mod writer;

pub fn init() -> Option<SnFramebufferDisplay> {
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(mut framebuffer) = framebuffer_response.framebuffers().next() {
            let display = SnFramebufferDisplay::new(&mut framebuffer);

            return Some(display);
        }
    }

    None
}