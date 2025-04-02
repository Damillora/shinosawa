use crate::{hal::x86_64::gdt, print, printk};
use conquer_once::spin::OnceCell;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

static IDT: OnceCell<InterruptDescriptorTable> = OnceCell::uninit();

// TODO: this is only initialized once for now
// Find a way to dynamically load interrupt handlers?
#[allow(static_mut_refs)]
pub fn init() {
    printk!("x86_64: initializing handlers");
    IDT.init_once(move || {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    });

    printk!("x86_64: loading interrupts");
    IDT.get().unwrap().load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    printk!("x86_64: breakpoint\n{:#?}", stack_frame);
}

// new
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("x86_64: double fault\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    printk!("x86_64: page fault");
    printk!("you tried to access address: {:?}", Cr2::read());
    printk!("error code: {:?}", error_code);
    printk!("{:#?}", stack_frame);

    panic!("page fault");
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
