use core::arch::asm;


pub enum Syscall {
    Read = 0,
    Write = 1,
    Fork = 10,
    Exit = 11,
    Max = 255,
}

pub struct SyscallError(u64);

pub fn write(str: &str)  {
    unsafe {
        asm!( // syscall function
             "syscall",
             in("rax") Syscall::Write as u64,
             in("rdi") str.as_ptr(), // First argument
             in("rsi") str.len()); // Second argument
    }

}

pub fn fork(
    func: extern "C" fn(usize) -> (),
    param: usize
) -> Result<u64, SyscallError> {

    let tid: u64;
    let errcode: u64;
    unsafe {
        asm!(
             "syscall",
             // rax = 0 indicates no error
             "cmp rax, 0",
             "jnz 2f",
             // rdi = 0 for new thread
             "cmp rdi, 0",
             "jnz 2f",
             // New thread
             "mov rdi, r9", // Function argument
             "call r8",
             "mov rax, 1", // exit_current_thread syscall
             "syscall",
             // New thread never leaves this asm block
             "2:",
             in("rax") Syscall::Fork as u64,
             in("r8") func,
             in("r9") param,
             lateout("rax") errcode,
             lateout("rdi") tid,
             out("rcx") _,
             out("r11") _);
    }
    if errcode != 0 {
        return Err(SyscallError(errcode));
    }
    Ok(tid)
}

pub fn exit() -> ! {
    unsafe {
        asm!("syscall",
             in("rax") Syscall::Exit as u64,
             options(noreturn));
    }
}