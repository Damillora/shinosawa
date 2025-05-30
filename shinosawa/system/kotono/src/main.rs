#![no_std]
#![no_main]

use core::{arch::asm, panic::PanicInfo};
use shinosawa_system_sysface::{_print, print, println, syscall};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn _start() -> ! {
    println!("shinosawa::system::kotono: starting init");

    extern "C" fn a(a: usize) {
        println!("shinosawa::system::kotono: after fork");
    }
    syscall::fork(a, 5);

    syscall::exit();
}