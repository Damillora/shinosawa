use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use crate::{hal::x86_64::gdt, printk};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        printk!("x86_64: initializing handlers");
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
    printk!("x86_64: loading interrupts");
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    printk!("x86_64: breakpoint\n{:#?}", stack_frame);
}

// new
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> ! 
{
    panic!("x86_64: double fault\n{:#?}", stack_frame);
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
