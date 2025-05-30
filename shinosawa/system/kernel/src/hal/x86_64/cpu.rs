use crate::printk;

use super::{gdt, interrupt, apic};

pub fn init() {
    printk!("x86_64: initializing APIC timer");
    apic::init();
    printk!("x86_64: initialing CPU tables");
    gdt::init();
    interrupt::init();
}

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct SnCpuContext {
    // These are pushed in the handler function
    pub _r15: usize,
    pub _r14: usize,
    pub _r13: usize,

    pub _r12: usize,
    pub _r11: usize,
    pub _r10: usize,
    pub _r9: usize,

    pub _r8: usize,
    pub _rbp: usize,
    pub _rsi: usize,
    pub rdi: usize,

    pub _rdx: usize,
    pub _rcx: usize,
    pub _rbx: usize,
    pub rax: usize,
    // Below is the exception stack frame pushed by the CPU on interrupt
    // Note: For some interrupts (e.g. Page fault), an error code is pushed here
    pub rip: usize,     // Instruction pointer
    pub cs: usize,      // Code segment
    pub rflags: usize,  // Processor flags
    pub rsp: usize,     // Stack pointer
    pub ss: usize,      // Stack segment
    // Here the CPU may push values to align the stack on a 16-byte boundary (for SSE)
}
impl SnCpuContext {
    pub fn instruction_pointer(&self) -> usize {
        self.rip
    }

    pub fn set_stack_pointer(&mut self, rsp: usize) {
        self.rsp = rsp;
    }

    pub fn set_ret_val_1(&mut self, rax: usize) {
        self.rax = rax;
    }

    pub fn set_arg_val_1(&mut self, rdi: usize) {
        self.rdi = rdi;
    }
}
pub unsafe fn set_context(context_addr: u64, function: u64, user_stack_end: u64, user: bool) {
    // Set context registers
    let context = unsafe {&mut *(context_addr as *mut SnCpuContext)};
    context.rip = function as usize; // Instruction pointer
    context.rsp = user_stack_end as usize; // Stack pointer
    context.rflags = 0x200; // Interrupts enabled

    if user {
        let (code_selector, data_selector) = gdt::get_user_segments();
        context.cs = code_selector.0 as usize;
        context.ss = data_selector.0 as usize;
    } else {
        let (code_selector, data_selector) = gdt::get_kernel_segments();
        context.cs = code_selector.0 as usize;
        context.ss = data_selector.0 as usize;
    }
}