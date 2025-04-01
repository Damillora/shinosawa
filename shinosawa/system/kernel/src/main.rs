#![no_std]
#![no_main]

use serial::SnSerialWriter;

/// Framebuffer module
mod fb;
/// Limine intrinsics
mod limine;

/// Logger module
mod logger;
/// Panic handler
mod panic;
/// Serial module
mod serial;
const VERSION: &str = "0.1.0";

pub fn kernel_main() {
    let display = fb::init().unwrap();
    let serial = unsafe { serial::init() };

    logger::init(display, serial);
    
    {
        printk!("shinosawa::system::kernel {}", VERSION);
        printk!("an operating system for those who find joy in things that don't go well,");
        printk!("written by someone least cut out for it.");
    }
}
