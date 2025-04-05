use crate::{hal::x86_64::gdt, printk, process::thread, syscall::SYSCALL_CONTROLLER};
use core::arch::{asm, naked_asm};

use super::cpu::SnCpuContext;

const MSR_STAR: usize = 0xc0000081;
const MSR_LSTAR: usize = 0xc0000082;
const MSR_FMASK: usize = 0xc0000084;
const MSR_KERNEL_GS_BASE: usize = 0xC0000102;

const SYSCALL_KERNEL_STACK_OFFSET: u64 = 512 * 2;

extern "C" fn dispatch_syscall(
    context_addr: u64,
    syscall_id: u64,
    arg1: u64, arg2: u64, arg3: u64
) {
    let context_ptr = context_addr as *mut SnCpuContext;
    let context = unsafe{&mut *context_ptr};

    // Set the CS and SS segment selectors
    let (code_selector, data_selector) =
          gdt::get_user_segments();
    context.cs = code_selector.0 as usize;
    context.ss = data_selector.0 as usize;

    let syscall_controller = SYSCALL_CONTROLLER.get().unwrap().read();
    syscall_controller.run_handler(syscall_id, context, arg1, arg2, arg3);
}

#[naked]
extern "C" fn handle_syscall() {
    // Empty for now
    unsafe {
        naked_asm!(
            // swap value in kernel and user GS base register
            "swapgs",
            "mov gs:{tss_syscall}, rsp", // save user RSP
            "mov rsp, gs:{tss_timer}", // load kernel RSP
            // Move stack pointer by two pages
            "sub rsp, {ks_offset}",

            "sub rsp, 8", // To be replaced with SS
            "push gs:{tss_syscall}", // user RSP
            "swapgs",
            // Here should switch stack to avoid messing with user stack
            "push r11", // Caller's RFLAGS
            "sub rsp, 8",  // CS
            "push rcx", // Caller's RIP

            // backup registers for sysretq
            
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

            "add rsp, 24", // Skip RIP, CS and RFLAGS
            "pop rsp", // Restore user stack
            // No need to pop SS

            // No need to pop SS
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
            tss_timer = const(0x24 + gdt::TIMER_IST_INDEX * 8),
            tss_syscall = const(0x24 + gdt::SYSCALL_IST_INDEX * 8),
            ks_offset = const(SYSCALL_KERNEL_STACK_OFFSET),
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
        // Set KERNEL_GS_BASE to TSS address
        asm!(
            // Want to move RDX into MSR but wrmsr takes EDX:EAX i.e. EDX
            // goes to high 32 bits of MSR, and EAX goes to low order bits
            // https://www.felixcloutier.com/x86/wrmsr
            "mov eax, edx",
            "shr rdx, 32", // Shift high bits into EDX
            "wrmsr",
            in("rcx") MSR_KERNEL_GS_BASE,
            in("rdx") gdt::tss_address()
        );
        // Set segment selectors when syscall ops are executed
        asm!(
            "xor rax, rax",
            "mov rdx, 0x230008",
            "wrmsr",
            in("rcx") MSR_STAR);
    }
}
