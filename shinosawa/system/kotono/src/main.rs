#![no_std]
#![no_main]

use core::{arch::asm, panic::PanicInfo};
#[macro_use]
use shinosawa_system_sysface::{_print, print, println, syscall};

#[unsafe(no_mangle)]
unsafe extern "C" fn main() -> ! {
    println!("shinosawa::system::kotono: starting init");

    extern "C" fn a(a: usize) {
        println!("shinosawa::system::kotono: we are at pid {:x}", a);

        syscall::exit();
    }
    let tid = syscall::fork(a, 5).unwrap();
    println!("shinosawa::system::kotono: we forked with tid {:?}", tid);

    syscall::exit();
}