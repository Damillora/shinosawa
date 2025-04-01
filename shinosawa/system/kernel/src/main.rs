#![no_std]
#![no_main]

use fb::writer::SnFramebufferWriter;

/// Framebuffer module
mod fb;
/// Limine protocol
mod limine;
/// Logger module
mod logger;
/// Panic handler
mod panic;
/// Serial module
mod serial;
const VERSION: &str = "0.1.0";

pub fn kernel_main() {
    let fb = fb::init().unwrap();
    
    let mut writer = SnFramebufferWriter::new(fb);
    writer.clear();

    let serial_writer = unsafe { serial::init() };

    logger::init(writer, serial_writer);

    {
        printk!("shinosawa::system::kernel {}", VERSION);
        printk!("an operating system for those who find joy in things that don't go well,");
        printk!("written by someone least cut out for it.");

        for f in 0..50 {
            printk!("loopaloop {}", f);
        }
    }
}
