use crate::printk;

use super::{gdt, interrupt, apic};

pub fn init() {
    printk!("x86_64: initializing APIC timer");
    apic::init();
    printk!("x86_64: initialing CPU tables");
    gdt::init();
    interrupt::init();
}