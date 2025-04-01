use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::{hal::x86_64::gdt, printk_sub};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); // new
        }

        idt
    };
}
// TODO: this is only initialized once for now
// Find a way to dynamically load interrupt handlers?
#[allow(static_mut_refs)]
pub fn init() {
    printk_sub!("x86_64", "initializing handlers");
    IDT.load();

    printk_sub!("x86_64", "loading interrupts");
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    printk_sub!("x86_64", "breakpoint\n{:#?}", stack_frame);
}

// new
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    printk_sub!("x86_64", "x86: double fault\n{:#?}", stack_frame);
    panic!("double fault reached");
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
