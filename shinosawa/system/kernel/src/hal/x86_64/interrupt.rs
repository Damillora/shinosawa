use core::arch::naked_asm;

use crate::{
    hal::x86_64::{apic::LOCAL_APIC, gdt}, memory::SnVirtAddr, print, printk
};
use conquer_once::spin::OnceCell;
use x86_64::{
    instructions::interrupts,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};


static IDT: OnceCell<InterruptDescriptorTable> = OnceCell::uninit();

pub static SCHEDULE: OnceCell<fn(usize) -> usize> = OnceCell::uninit();

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    ApicError = 0xfd,
    ApicTimer = 0xfe,
    ApicSpurious = 0xff,
}
pub enum InterruptStackIndex {
    Timer = 0x01,
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
        unsafe {
            idt[InterruptIndex::ApicTimer.as_u8()]
                .set_handler_fn(timer_interrupt_handler_preempt)
                .set_stack_index(gdt::TIMER_IST_INDEX);;
        }
        unsafe {
            idt.general_protection_fault
                .set_handler_fn(general_protection_fault_handler)
                .set_stack_index(gdt::GENERAL_PROTECTION_FAULT_IST_INDEX);
        }
        idt
    });

    printk!("x86_64: loading interrupts");
    IDT.get().unwrap().load();

    printk!("x86_64: we will start receiving interrupts!");
    x86_64::instructions::interrupts::enable(); // new
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

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
    printk!("x86_64: general protection fault");

    printk!("{:#?}", stack_frame);

    panic!("general protection fault");
}

extern "C" fn timer_interrupt_handler(context_addr: usize) -> usize{
    if SCHEDULE.is_initialized() {
        let next_stack = (SCHEDULE.get().unwrap())(context_addr);
        
        let mut lapic = LOCAL_APIC.get().unwrap().lock();
        unsafe { lapic.end_of_interrupt() };

        return next_stack;
    } else {
        let mut lapic = LOCAL_APIC.get().unwrap().lock();
        unsafe { lapic.end_of_interrupt() };

        return 0;
    }
}

#[naked]
pub extern "x86-interrupt" fn timer_interrupt_handler_preempt(_stack_frame: InterruptStackFrame) {
    unsafe {
        naked_asm!(
            // Disable interrupts
            "cli",
            // Push registers
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",

            "push rdi",
            "push rsi",
            "push rbp",
            "push r8",

            "push r9",
            "push r10",
            "push r11",
            "push r12",

            "push r13",
            "push r14",
            "push r15",

            // First argument in rdi with C calling convention
            "mov rdi, rsp",
            // Call the hander function
            "call {handler}",

            // New: stack pointer is in RAX
            "cmp rax, 0",
            "je 2f",        // if rax != 0 {
            "mov rsp, rax", //   rsp = rax;
            "2:",           // }

            // Pop scratch registers
            "pop r15",
            "pop r14",
            "pop r13",

            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",

            "pop r8",
            "pop rbp",
            "pop rsi",
            "pop rdi",

            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",
            // Enable interrupts
            "sti",
            // Interrupt return
            "iretq",
            // Note: Getting the handler pointer here using `sym` operand, because
            // an `in` operand would clobber a register that we need to save, and we
            // can't have two asm blocks
            handler = sym timer_interrupt_handler,
        );
    }
}

/// Number of bytes needed to store a Context struct
pub const INTERRUPT_CONTEXT_SIZE: usize = 20 * 8;

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}

pub fn without_interrupts<T: FnOnce() -> ()>(a: T) {
    interrupts::without_interrupts(a);
}

pub fn set_interrupt_stack_table(index: usize, stack_end: SnVirtAddr) {
    gdt::set_interrupt_stack_table(index, stack_end);
}
