use crate::{hal::x86_64::{apic::LOCAL_APIC, gdt}, print, printk};
use conquer_once::spin::OnceCell;
use x86_64::{instructions::interrupts, structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode}};

static IDT: OnceCell<InterruptDescriptorTable> = OnceCell::uninit();

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    ApicError = 0xfd,
    ApicTimer = 0xfe,
    ApicSpurious = 0xff,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

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
        idt[InterruptIndex::ApicTimer.as_u8()]
            .set_handler_fn(timer_interrupt_handler); // new
        idt
    });

    printk!("x86_64: loading interrupts");
    IDT.get().unwrap().load();

    printk!("x86_64: we will start receiving interrupts!");
    x86_64::instructions::interrupts::enable();     // new
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
    let mut lapic =LOCAL_APIC.get().unwrap().lock();
    unsafe { lapic.end_of_interrupt() };
    // lapic.end_of_interrupt();
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}

pub fn without_interrupts<T: FnOnce() -> ()>(a: T) {
    interrupts::without_interrupts(a);
}