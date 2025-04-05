use core::slice;

use conquer_once::spin::OnceCell;
use spin::RwLock;

use crate::{hal::interface::cpu::SnCpuContext, print, printk, println, process};

pub const SYSCALL_INDEXES: usize = 32;

// Currently registered syscalls:
// 0: writer

pub enum Syscall {
    Read = 0,
    Write = 1,
    Fork = 10,
    Exit = 11,
    Max = 255,
}
pub struct SyscallHandler {
    handler: fn(&mut SnCpuContext, u64, u64, u64),
}

pub struct SyscallController {
    handlers: [Option<SyscallHandler>; SYSCALL_INDEXES],
}

impl SyscallController {
    pub fn new() -> SyscallController{
        SyscallController {
            handlers: [const { None }; SYSCALL_INDEXES],
        }
    }

    pub fn set_handler(&mut self, idx: u64,  handler: fn(&mut SnCpuContext, u64, u64, u64)) {
        self.handlers[idx as usize] = Some(SyscallHandler { handler: handler });
    }

    pub fn run_handler(&self, idx: u64, ctx: &mut SnCpuContext, arg1: u64, arg2: u64, arg3: u64) {
        if let Some(handler) = &self.handlers[idx as usize] {
            (handler.handler)(ctx, arg1, arg2, arg3);
        }
    }
}

pub static SYSCALL_CONTROLLER: OnceCell<RwLock<SyscallController>> = OnceCell::uninit();
pub fn init() {
    printk!("syscall: initializing general syscall handler");
    SYSCALL_CONTROLLER.init_once(move || RwLock::new(SyscallController::new()) );

    let mut controller = SYSCALL_CONTROLLER.get().unwrap().write();
    controller.set_handler(Syscall::Write as u64, write);
    controller.set_handler(Syscall::Fork as u64, fork);
    controller.set_handler(Syscall::Exit as u64, exit);
}

fn write(ctx: &mut SnCpuContext, ptr: u64, len: u64, arg3: u64) {
    // Check all inputs: Does ptr -> ptr+len lie entirely in user address space?
    if len == 0 {
        return;
    }
    // Convert raw pointer to a slice
    let u8_slice = unsafe {slice::from_raw_parts(ptr as *mut u8, len as usize)};

    if let Ok(s) = str::from_utf8(u8_slice) {
        print!("{}", s);
    } // else error
}

fn fork(ctx: &mut SnCpuContext, _arg1: u64, _arg2: u64, _arg3: u64) {
    process::thread::fork_current_thread(ctx);
}

fn exit(ctx: &mut SnCpuContext, _arg1: u64, _arg2: u64, _arg3: u64) {
    process::thread::exit_current_thread(ctx);
}