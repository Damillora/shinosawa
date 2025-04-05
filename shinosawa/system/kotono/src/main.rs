#![no_std]
#![no_main]

use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn _start() -> ! {
    for f in 0..100 {
        unsafe {
            asm!(
                // "mov rdi, 1", // write
                "syscall"
            );
        }
        
    }
    loop {}
}