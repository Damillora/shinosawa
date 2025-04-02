extern crate alloc;

use alloc::vec::Vec;

use conquer_once::spin::OnceCell;
use spin::rwlock::RwLock;
use alloc::{boxed::Box, collections::vec_deque::VecDeque};

use crate::{hal::interface::interrupt::{InterruptStackIndex, INTERRUPT_CONTEXT_SIZE, SCHEDULE}, memory::SnVirtAddr, printk};

#[derive(Debug)]
struct Thread {
    kernel_stack: Vec<u8>,
    user_stack: Vec<u8>,
    kernel_stack_end: u64, // This address goes in the TSS
    user_stack_end: u64,
    context: u64, // Address of Context on kernel stack
}

static RUNNING_QUEUE: OnceCell<RwLock<VecDeque<Box<Thread>>>> = OnceCell::new(RwLock::new(VecDeque::new()));

static CURRENT_THREAD: RwLock<Option<Box<Thread>>> = RwLock::new(None);

const KERNEL_STACK_SIZE: u64 = 4096 * 2;
const USER_STACK_SIZE: u64 = 4096 * 5;

pub fn new_kernel_thread(function: fn()->()) {
    printk!("process: spawning new kernel thread {:x}", function as u64);
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

    unsafe { crate::hal::interface::cpu::set_context(new_thread.context, function as u64, new_thread.user_stack_end, false) };

    crate::hal::interface::interrupt::without_interrupts(|| {
        RUNNING_QUEUE.get().unwrap().write().push_back(new_thread);
    });
}

pub fn new_user_thread(function: fn()->()) {
    printk!("process: spawning new user thread {:x}", function as u64);
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

    unsafe { crate::hal::interface::cpu::set_context(new_thread.context, function as u64, new_thread.user_stack_end, true)};

    crate::hal::interface::interrupt::without_interrupts(|| {
        RUNNING_QUEUE.get().unwrap().write().push_back(new_thread);
    });
}


/// Adds a thread to the front of the running queue
/// so it will be scheduled next
pub fn schedule_thread(thread: Box<Thread>) {
    // Turn off interrupts while modifying process table
    crate::hal::interface::interrupt::without_interrupts(|| {
        RUNNING_QUEUE.get().unwrap().write().push_front(thread);
    });
}

fn schedule_next(context_addr: usize) -> usize {
    let mut running_queue = RUNNING_QUEUE.get().unwrap().write();
    let mut current_thread = CURRENT_THREAD.write();

    if let Some(mut thread) = current_thread.take() {
        // Save the location of the Context struct
        thread.context = context_addr as u64;
        // Put to the back of the queue
        running_queue.push_back(thread);
    }
    
    *current_thread = running_queue.pop_front();
    match current_thread.as_ref() {
        Some(thread) => {
            // Set the kernel stack for the next interrupt
            crate::hal::interface::interrupt::set_interrupt_stack_table(
              InterruptStackIndex::Timer as usize,
              SnVirtAddr::new(thread.kernel_stack_end));
            // Point the stack to the new context
            thread.context as usize
          },
        None => 0  // Timer handler won't modify stack
    }
}

pub fn init() {
    printk!("process: setting the scheduler");
    SCHEDULE.init_once(move || schedule_next);
}