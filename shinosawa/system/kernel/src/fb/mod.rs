use limine::request::FramebufferRequest;


#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();


pub fn init() {
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
            for i in 0..framebuffer.height() {
                for j in 0..framebuffer.width() {
                    // Calculate the pixel offset using the framebuffer information we obtained above.
                    let pixel_offset = i * framebuffer.pitch() + j * 4;
    
                    // Write 0xFFFFFFFF to the provided pixel offset to fill it white.
                    unsafe {
                        framebuffer
                            .addr()
                            .add(pixel_offset as usize)
                            .cast::<u32>()
                            .write(0xFFFFFFFF)
                    };

                }

            }
        }
    }
}