use crate::{hal::x86_64::gdt, printk_sub};

pub fn init() {
    printk_sub!("x86_64", "initializing CPU");
    
    gdt::init();
}
