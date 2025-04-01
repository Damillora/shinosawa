#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(tests::test_runner)]
#![reexport_test_harness_main = "kernel_test_main"]

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
/// Tests
mod tests;
/// Abstraction over architecture specific stuff
mod hal;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn kernel_main() {
    let display = fb::init().unwrap();
    let serial = unsafe { serial::init() };

    logger::init(display, serial);

    {
        printk!("shinosawa::system::kernel {}", VERSION);
        printk!("an operating system for those who find joy in things that don't go well,");
        printk!("written by someone least cut out for it.");
    }

    #[cfg(test)]
    {
        printk!("tests has been enabled. running them now.");
        use hal::interface::instruct::hcf;

        kernel_test_main();

        hcf();
    }
}

#[test_case]
fn trivial_assertion() {
    printk!("trivial assertion... ");
    assert_eq!(1, 1);
    printk!("[ok]");
}
