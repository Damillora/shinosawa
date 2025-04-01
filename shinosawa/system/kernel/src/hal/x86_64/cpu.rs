use super::{gdt, interrupt};

pub fn init() {
    gdt::init();
    interrupt::init();
}