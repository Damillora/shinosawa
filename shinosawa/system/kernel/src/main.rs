#![no_std]
#![no_main]

mod fb;
mod limine;
mod logger;
mod panic;
const VERSION: &str = "0.1.0";

pub fn kernel_main() {
    let display = fb::init().unwrap();

    logger::init(display);
    
    {
        printk!("shinosawa::system::kernel {}", VERSION);
        printk!("an operating system for those who find joy in things that don't go well,");
        printk!("written by someone least cut out for it.");

        for f in 0..50 {
            printk!("loopaloop {}", f);
        }
    }
}
