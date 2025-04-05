use crate::{printk, process::thread};
use core::arch::{asm, naked_asm};

const MSR_STAR: usize = 0xc0000081;
const MSR_LSTAR: usize = 0xc0000082;
const MSR_FMASK: usize = 0xc0000084;

extern "C" fn sys_write() {
    printk!("x86_64::syscall: write");
}

#[naked]
extern "C" fn handle_syscall() {
    // Empty for now
    unsafe {
        naked_asm!(
            // Here should switch stack to avoid messing with user stack
            "push r11", // Caller's RFLAGS
            "sub rsp, 8",  // CS
            "push rcx", // Caller's RIP
            // backup registers for sysretq
            "push rcx",
            "push r11",
            "push rbp",
            "push rbx", // save callee-saved registers
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            // Call the rust handler
            "call {sys_write}",
            "pop r15", // restore callee-saved registers
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbx",
            "pop rbp", // restore stack and registers for sysretq
            "pop r11",
            "pop rcx",

            "cmp rcx, {user_code_start}",
            "jl 2f", // rip < USER_CODE_START
            "cmp rcx, {user_code_end}",
            "jge 2f", // rip >= USER_CODE_END
            "sysretq", // back to userland

            "2:", // kernel code return
            "push r11",
            "popf", // Set RFLAGS
            "jmp rcx",
            sys_write = sym sys_write,
            user_code_start = const(thread::USER_CODE_START),
            user_code_end = const(thread::USER_CODE_END),
        );
    }
}

pub fn init() {
    printk!("x86_64::syscall: initializing syscall interface");
    let handler_addr = handle_syscall as *const () as u64;
    unsafe {
        // Enable syscall and sysret ops
        asm!("mov ecx, 0xC0000080",
            "rdmsr",
            "or eax, 1",
            "wrmsr");
        // Use FMASK to disable interrupts during syscall
        asm!("xor rdx, rdx",
            "mov rax, 0x200",
            "wrmsr",
            in("rcx") MSR_FMASK);
        // Set LSTAR to handler
        asm!("mov rdx, rax",
            "shr rdx, 32",
            "wrmsr",
            in("rax") handler_addr,
            in("rcx") MSR_LSTAR);
        // Set segment selectors when syscall ops are executed
        asm!(
            "xor rax, rax",
            "mov rdx, 0x230008",
            "wrmsr",
            in("rcx") MSR_STAR);
    }
}
