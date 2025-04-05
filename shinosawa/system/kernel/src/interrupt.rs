use conquer_once::spin::OnceCell;
use spin::RwLock;

use crate::printk;


pub const FREE_VECTORS_START: u8 = 0x40;
pub const FREE_VECTORS: usize = 0x20;

pub struct InterruptHandler {
    handler: fn(),
}

pub struct InterruptController {
    handlers: [Option<InterruptHandler>; FREE_VECTORS],
}

impl InterruptController {
    pub fn new() -> InterruptController{
        InterruptController {
            handlers: [const { None }; FREE_VECTORS],
        }
    }

    pub fn set_handler(&mut self, idx: usize,  handler: fn()) {
        self.handlers[idx] = Some(InterruptHandler { handler: handler });
    }

    pub fn run_handler(&self, idx: usize) {
        if let Some(handler) = &self.handlers[idx] {
            (handler.handler)();
        }
    }
}

pub static INTERRUPT_CONTROLLER: OnceCell<RwLock<InterruptController>> = OnceCell::uninit();
pub fn init() {
    printk!("interrupt: initializing general interrupt controller");
    INTERRUPT_CONTROLLER.init_once(move || RwLock::new(InterruptController::new()) );
}