#![no_std]
#![no_main]

use core::fmt::Write;

use log::LOGGER;

mod fb;
mod limine;
mod log;
mod panic;
const VERSION: &str = "0.1.0";

pub fn kernel_main() {
    fb::init();

    {
        printk!("shinosawa::system::kernel {}", VERSION);
        printk!(
            "an operating system for those who find joy in things that don't go well,"
        );
        printk!("written by someone least cut out for it.");
    }
}
