use crate::printk;

use super::{gdt, interrupt, apic};

pub fn init() {
    printk!("x86_64: initializing APIC timer");
    apic::init();
    printk!("x86_64: initialing CPU tables");
    gdt::init();
    interrupt::init();
}

#[derive(Debug)]
#[repr(packed)]
pub struct SnCpuContext {
    // These are pushed in the handler function
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,

    pub r12: usize,
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,

    pub r8: usize,
    pub rbp: usize,
    pub rsi: usize,
    pub rdi: usize,

    pub rdx: usize,
    pub rcx: usize,
    pub rbx: usize,
    pub rax: usize,
    // Below is the exception stack frame pushed by the CPU on interrupt
    // Note: For some interrupts (e.g. Page fault), an error code is pushed here
    rip: usize,     // Instruction pointer
    cs: usize,      // Code segment
    rflags: usize,  // Processor flags
    rsp: usize,     // Stack pointer
    ss: usize,      // Stack segment
    // Here the CPU may push values to align the stack on a 16-byte boundary (for SSE)
}