#![no_std]
#![no_main]

use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn _start() -> ! {
    let s = "hello";
    unsafe {
        asm!("mov rax, 1", // syscall function
             "syscall",
             in("rdi") s.as_ptr(), // First argument
             in("rsi") s.len()); // Second argument
    }
    loop {}
}