use core::slice;

use conquer_once::spin::OnceCell;
use spin::RwLock;

use crate::{printk, println};


pub const FREE_VECTORS_START: u8 = 0x40;
pub const FREE_VECTORS: usize = 0x20;

pub struct SyscallHandler {
    handler: fn(u64, u64, u64),
}

pub struct SyscallController {
    handlers: [Option<SyscallHandler>; FREE_VECTORS],
}

impl SyscallController {
    pub fn new() -> SyscallController{
        SyscallController {
            handlers: [const { None }; FREE_VECTORS],
        }
    }

    pub fn set_handler(&mut self, idx: u64,  handler: fn(u64, u64, u64)) {
        self.handlers[idx as usize] = Some(SyscallHandler { handler: handler });
    }

    pub fn run_handler(&self, idx: u64, arg1: u64, arg2: u64, arg3: u64) {
        if let Some(handler) = &self.handlers[idx as usize] {
            (handler.handler)(arg1, arg2, arg3);
        }
    }
}

pub static SYSCALL_CONTROLLER: OnceCell<RwLock<SyscallController>> = OnceCell::uninit();
pub fn init() {
    printk!("syscall: initializing general syscall handler");
    SYSCALL_CONTROLLER.init_once(move || RwLock::new(SyscallController::new()) );

    let mut controller = SYSCALL_CONTROLLER.get().unwrap().write();
    controller.set_handler(1, write);
}


fn write(ptr: u64, len: u64, arg3: u64) {
    // Check all inputs: Does ptr -> ptr+len lie entirely in user address space?
    if len == 0 {
        return;
    }
    // Convert raw pointer to a slice
    let u8_slice = unsafe {slice::from_raw_parts(ptr as *mut u8, len as usize)};

    if let Ok(s) = str::from_utf8(u8_slice) {
        println!("{}", s);
    } // else error

}