use crate::{printk, process::thread, syscall::SYSCALL_CONTROLLER};
use core::arch::{asm, naked_asm};

const MSR_STAR: usize = 0xc0000081;
const MSR_LSTAR: usize = 0xc0000082;
const MSR_FMASK: usize = 0xc0000084;

extern "C" fn dispatch_syscall(
    context_addr: u64,
    syscall_id: u64,
    arg1: u64, arg2: u64, arg3: u64
) {
    let syscall_controller = SYSCALL_CONTROLLER.get().unwrap().read();
    syscall_controller.run_handler(syscall_id, arg1, arg2, arg3);
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

            "mov r8, rdx", // Fifth argument <- Syscall third argument
            "mov rcx, rsi", // Fourth argument <- Syscall second argument
            "mov rdx, rdi", // Third argument <- Syscall first argument
            "mov rsi, rax", // Second argument is the syscall number
            "mov rdi, rsp", // First argument is the Context address
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
            sys_write = sym dispatch_syscall,
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
