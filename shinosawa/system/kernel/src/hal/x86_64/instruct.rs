use core::arch::asm;

pub fn hcf() -> ! {
    loop {
        halt();
    }
}

pub fn halt() {
    unsafe {
        asm!("hlt");
    }
}