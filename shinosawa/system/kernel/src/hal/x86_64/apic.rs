use acpi::InterruptModel;
use alloc::boxed::Box;

use crate::{acpi::{SnHardwareInfo, HARDWARE_INFO}, printk};


pub fn init() {
    printk!("x86_64::apic: initializing");

    let hw_info = HARDWARE_INFO.get().unwrap();
    if let InterruptModel::Apic(apic) = &hw_info.interrupt_model {
        printk!("x86_64::apic: this system has APIC");
    } else {
        printk!("x86_64::apic: this system does not use APIC, apparently");
    }

}