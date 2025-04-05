// say no to std and main
#![no_std]
#![no_main]
// custom test runner
#![feature(custom_test_frameworks)]
#![test_runner(tests::test_runner)]
#![reexport_test_harness_main = "kernel_test_main"]

#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(naked_functions)]

use alloc::vec::Vec;
use hal::x86_64::instruct::hcf;
use logger::{clean_buffer, logbuf::SnLogBuffer};

extern crate alloc;

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
/// Memory management
mod memory;
/// ACPI
mod acpi;
/// Process management
mod process;
/// Interrupt management
mod interrupt;
/// Syscall controller
mod syscall;
/// Device drivers
mod drivers;
/// Filesystem
mod fs;
/// Executable loaders
mod loader;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init() {
    logger::init();
    crate::hal::interface::paging::init();
    crate::memory::alloc::init();

    let buffer = SnLogBuffer::new();
    logger::set_buffer(buffer);

    {
        printk!("shinosawa::system::kernel {}", VERSION);
        printk!("an operating system for those who find joy in things that don't go well,");
        printk!("written by someone least cut out for it.");
    }

    let serial = unsafe { serial::init() };
    logger::set_serial(serial);
    let display = fb::init().unwrap();
    logger::set_fb(display);
    clean_buffer();
    
    crate::acpi::init();

    crate::hal::interface::cpu::init();
    crate::interrupt::init();

    crate::process::thread::init();

    // We can *actually* start a kernel process now.
    crate::process::thread::new_kernel_thread(kernel_main);

    hcf();
}

pub fn kernel_main() {
    // PS/2 driver
    crate::drivers::ps2_keyboard::init();

    // VFS system
    crate::fs::vfs::init();
    // Init dummy example filesystem
    let example_fs = crate::fs::dummy::new_example_filesystem();
    crate::fs::vfs::attach("SNSW:", example_fs);

    // Initialize syscall for CPU
    crate::hal::interface::syscall::init();
    // Initialize syscall controller
    crate::syscall::init();

    let file = crate::fs::vfs::find("SNSW:/shinosawa/system/kotono").unwrap();
    let len = file.len();
    let mut buf = [0; 10000];

    file.read(&mut buf).unwrap();

    let kotono = crate::loader::elf::load_elf(&buf).unwrap();

    // We can *actually* start a user process now.
    crate::process::thread::new_user_thread(kotono);
    #[cfg(test)]
    {
        printk!("tests has been enabled. running them now.");

        kernel_test_main();
    }
    
    {
        use hal::interface::instruct::hcf;
        
        printk!("kernel init done! we'll wait here.");
        hcf();
    }
}

#[test_case]
fn trivial_assertion() {
    printk!("trivial assertion... ");
    assert_eq!(1, 1);
    printk!("[ok]");
}