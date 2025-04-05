use core::arch::naked_asm;

use crate::{
    hal::x86_64::{apic::LOCAL_APIC, gdt}, interrupt::{FREE_VECTORS_START, INTERRUPT_CONTROLLER}, memory::SnVirtAddr, print, printk
};
use conquer_once::spin::OnceCell;
use x86_64::{
    instructions::interrupts,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

use super::apic;


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
        unsafe {
            idt.page_fault
                .set_handler_fn(page_fault_handler)
                .set_stack_index(gdt::PAGE_FAULT_IST_INDEX);; 
            }
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        unsafe {
            idt[InterruptIndex::ApicTimer.as_u8()]
                .set_handler_fn(timer_interrupt_handler_preempt)
                .set_stack_index(gdt::TIMER_IST_INDEX);
        }
        unsafe {
            idt.general_protection_fault
                .set_handler_fn(general_protection_fault_handler)
                .set_stack_index(gdt::GENERAL_PROTECTION_FAULT_IST_INDEX);
        }
        unsafe { idt[FREE_VECTORS_START + 0x01].set_handler_fn(platform_handler_01).set_stack_index(gdt::PLATFORM_HANDLER_IST_INDEX) };
        unsafe { idt[FREE_VECTORS_START + 0x02].set_handler_fn(platform_handler_02).set_stack_index(gdt::PLATFORM_HANDLER_IST_INDEX) };
        idt[FREE_VECTORS_START + 0x03].set_handler_fn(platform_handler_03);
        idt[FREE_VECTORS_START + 0x04].set_handler_fn(platform_handler_04);
        idt[FREE_VECTORS_START + 0x05].set_handler_fn(platform_handler_05);
        idt[FREE_VECTORS_START + 0x06].set_handler_fn(platform_handler_06);
        idt[FREE_VECTORS_START + 0x07].set_handler_fn(platform_handler_07);
        idt[FREE_VECTORS_START + 0x08].set_handler_fn(platform_handler_08);
        idt[FREE_VECTORS_START + 0x09].set_handler_fn(platform_handler_09);
        idt[FREE_VECTORS_START + 0x0a].set_handler_fn(platform_handler_0a);
        idt[FREE_VECTORS_START + 0x0b].set_handler_fn(platform_handler_0b);
        idt[FREE_VECTORS_START + 0x0c].set_handler_fn(platform_handler_0c);
        idt[FREE_VECTORS_START + 0x0d].set_handler_fn(platform_handler_0d);
        idt[FREE_VECTORS_START + 0x0e].set_handler_fn(platform_handler_0e);
        idt[FREE_VECTORS_START + 0x0f].set_handler_fn(platform_handler_0f);
        idt[FREE_VECTORS_START + 0x10].set_handler_fn(platform_handler_10);
        idt[FREE_VECTORS_START + 0x11].set_handler_fn(platform_handler_11);
        idt[FREE_VECTORS_START + 0x12].set_handler_fn(platform_handler_12);
        idt[FREE_VECTORS_START + 0x13].set_handler_fn(platform_handler_13);
        idt[FREE_VECTORS_START + 0x14].set_handler_fn(platform_handler_14);
        idt[FREE_VECTORS_START + 0x15].set_handler_fn(platform_handler_15);
        idt[FREE_VECTORS_START + 0x16].set_handler_fn(platform_handler_16);
        idt[FREE_VECTORS_START + 0x17].set_handler_fn(platform_handler_17);
        idt[FREE_VECTORS_START + 0x18].set_handler_fn(platform_handler_18);
        idt[FREE_VECTORS_START + 0x19].set_handler_fn(platform_handler_19);
        idt[FREE_VECTORS_START + 0x1a].set_handler_fn(platform_handler_1a);
        idt[FREE_VECTORS_START + 0x1b].set_handler_fn(platform_handler_1b);
        idt[FREE_VECTORS_START + 0x1c].set_handler_fn(platform_handler_1c);
        idt[FREE_VECTORS_START + 0x1d].set_handler_fn(platform_handler_1d);
        idt[FREE_VECTORS_START + 0x1e].set_handler_fn(platform_handler_1e);
        idt[FREE_VECTORS_START + 0x1f].set_handler_fn(platform_handler_1f);

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

fn platform_handler(idx: u8) {
    let interrupt_controller = INTERRUPT_CONTROLLER.get().unwrap().read();

    interrupt_controller.run_handler(idx as usize);
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

macro_rules! platform_handler {
    ($name:ident, $number:literal) => {
        extern "x86-interrupt" fn $name(
            stack_frame: InterruptStackFrame,
        ) {
            platform_handler($number);

            let mut lapic = LOCAL_APIC.get().unwrap().lock();
            unsafe { lapic.end_of_interrupt() };
        }
    };
}

platform_handler!(platform_handler_00, 0x00);
platform_handler!(platform_handler_01, 0x01);
platform_handler!(platform_handler_02, 0x02);
platform_handler!(platform_handler_03, 0x03);
platform_handler!(platform_handler_04, 0x04);
platform_handler!(platform_handler_05, 0x05);
platform_handler!(platform_handler_06, 0x06);
platform_handler!(platform_handler_07, 0x07);
platform_handler!(platform_handler_08, 0x08);
platform_handler!(platform_handler_09, 0x09);
platform_handler!(platform_handler_0a, 0x0a);
platform_handler!(platform_handler_0b, 0x0b);
platform_handler!(platform_handler_0c, 0x0c);
platform_handler!(platform_handler_0d, 0x0d);
platform_handler!(platform_handler_0e, 0x0e);
platform_handler!(platform_handler_0f, 0x0f);
platform_handler!(platform_handler_10, 0x10);
platform_handler!(platform_handler_11, 0x11);
platform_handler!(platform_handler_12, 0x12);
platform_handler!(platform_handler_13, 0x13);
platform_handler!(platform_handler_14, 0x14);
platform_handler!(platform_handler_15, 0x15);
platform_handler!(platform_handler_16, 0x16);
platform_handler!(platform_handler_17, 0x17);
platform_handler!(platform_handler_18, 0x18);
platform_handler!(platform_handler_19, 0x19);
platform_handler!(platform_handler_1a, 0x1a);
platform_handler!(platform_handler_1b, 0x1b);
platform_handler!(platform_handler_1c, 0x1c);
platform_handler!(platform_handler_1d, 0x1d);
platform_handler!(platform_handler_1e, 0x1e);
platform_handler!(platform_handler_1f, 0x1f);


/// Number of bytes needed to store a Context struct
pub const INTERRUPT_CONTEXT_SIZE: usize = 20 * 8;

pub fn enable_irq(irq: u8) {
    apic::enable_irq(1);
}

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
