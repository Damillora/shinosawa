use crate::printk;

use super::{gdt, interrupt};

pub fn init() {
    printk!("x86_64: initialing CPU tables");
    gdt::init();
    interrupt::init();
}