extern crate alloc;

use alloc::vec::Vec;

use conquer_once::spin::OnceCell;
use spin::rwlock::RwLock;
use alloc::{boxed::Box, collections::vec_deque::VecDeque};

use crate::{hal::interface::interrupt::INTERRUPT_CONTEXT_SIZE, memory::SnVirtAddr};

struct Thread {
    kernel_stack: Vec<u8>,
    user_stack: Vec<u8>,
    kernel_stack_end: u64, // This address goes in the TSS
    user_stack_end: u64,
    context: u64, // Address of Context on kernel stack
}


static RUNNING_QUEUE: OnceCell<RwLock<VecDeque<Box<Thread>>>> = OnceCell::new(RwLock::new(VecDeque::new()));

static CURRENT_THREAD: RwLock<Option<Thread>> = RwLock::new(None);

const KERNEL_STACK_SIZE: u64 = 4096 * 2;
const USER_STACK_SIZE: u64 = 4096 * 5;

pub fn new_kernel_thread(function: fn()->()) {
    let new_thread = {
        let kernel_stack = Vec::with_capacity(KERNEL_STACK_SIZE as usize);
        let kernel_stack_end = (SnVirtAddr::from_ptr(kernel_stack.as_ptr())
                               + KERNEL_STACK_SIZE).as_u64();
        let user_stack = Vec::with_capacity(USER_STACK_SIZE as usize);
        let user_stack_end = (SnVirtAddr::from_ptr(user_stack.as_ptr())
                              + USER_STACK_SIZE).as_u64();
        let context = kernel_stack_end - INTERRUPT_CONTEXT_SIZE as u64;

        Box::new(Thread {
            kernel_stack,
            user_stack,
            kernel_stack_end,
            user_stack_end,
            context})
    };

    unsafe { crate::hal::interface::cpu::set_context(new_thread.context, function as u64, new_thread.user_stack_end) };

    crate::hal::interface::interrupt::without_interrupts(|| {
        RUNNING_QUEUE.get().unwrap().write().push_back(new_thread);
    });
}
