extern crate alloc;

use core::arch::asm;

use alloc::sync::Arc;
use alloc::vec::Vec;

use alloc::{boxed::Box, collections::vec_deque::VecDeque};
use conquer_once::spin::OnceCell;
use spin::rwlock::RwLock;

use crate::hal::interface::cpu::SnCpuContext;
use crate::memory::{KERNEL_STACK_SIZE, USER_STACK_SIZE};
use crate::{
    hal::interface::{
        interrupt::{INTERRUPT_CONTEXT_SIZE, InterruptStackIndex, SCHEDULE},
        paging,
    },
    loader::SnExecutable,
    memory::{SnPhysAddr, SnVirtAddr},
    printk,
};

use super::process::Process;

// Allocate pages for the user stack
const USER_STACK_START: u64 = 0x5002000;

/// Lowest address that user code can be loaded into
pub const USER_CODE_START: u64 = 0x20_0000;
/// Exclusive upper limit for user code or data
pub const USER_CODE_END: u64 = 0x5000_0000;

static RUNNING_QUEUE: OnceCell<RwLock<VecDeque<Box<Thread>>>> =
    OnceCell::new(RwLock::new(VecDeque::new()));

static CURRENT_THREAD: RwLock<Option<Box<Thread>>> = RwLock::new(None);

static THREAD_COUNTER: OnceCell<RwLock<u64>> = OnceCell::new(RwLock::new(0));
static PROCESS_COUNTER: OnceCell<RwLock<u64>> = OnceCell::new(RwLock::new(0));

struct Thread {
    id: u64,
    process: Arc<Process>,
    kernel_stack: Vec<u8>,
    kernel_stack_end: u64, // This address goes in the TSS
    user_stack_end: u64,
    context: u64, // Address of Context on kernel stack

    page_table_addr: u64,
}
impl Drop for Thread {
    fn drop(&mut self) {
        let _ =
            crate::hal::interface::paging::free_user_stack(SnVirtAddr::new(self.user_stack_end));
    }
}

pub fn new_thread_id() -> u64 {
    crate::hal::interface::interrupt::without_interrupts(|| {
        let mut counter = THREAD_COUNTER.get().unwrap().write();
        *counter += 1;
        *counter
    })
}

pub fn new_process_id() -> u64 {
    crate::hal::interface::interrupt::without_interrupts(|| {
        let mut counter = PROCESS_COUNTER.get().unwrap().write();
        *counter += 1;
        *counter
    })
}

pub fn new_kernel_thread(function: fn() -> ()) {
    printk!("process: spawning new kernel thread {:x}", function as u64);
    let new_thread = {
        let thread_id = new_thread_id();
        let kernel_stack =
            Vec::with_capacity(KERNEL_STACK_SIZE as usize + USER_STACK_SIZE as usize);
        let kernel_stack_end =
            (SnVirtAddr::from_ptr(kernel_stack.as_ptr()) + KERNEL_STACK_SIZE).as_u64();
        let user_stack_end = kernel_stack_end + USER_STACK_SIZE;

        let context = kernel_stack_end - INTERRUPT_CONTEXT_SIZE as u64;

        Box::new(Thread {
            id: thread_id,
            process: Arc::new(Process {
                id: new_process_id(),
                page_table_phys_addr: 0,
            }),
            kernel_stack,
            kernel_stack_end,
            user_stack_end,
            context,
            page_table_addr: 0,
        })
    };

    unsafe {
        crate::hal::interface::cpu::set_context(
            new_thread.context,
            function as u64,
            new_thread.user_stack_end,
            false,
        )
    };

    crate::hal::interface::interrupt::without_interrupts(|| {
        RUNNING_QUEUE.get().unwrap().write().push_back(new_thread);
    });
}

pub fn new_user_thread<T: SnExecutable>(executable: T) {
    printk!(
        "process: spawning new user thread {:x}",
        executable.entry_point().as_u64()
    );

    let (user_stack, user_stack_end) = paging::get_user_thread_stack(executable.page_table_phys().as_u64()).unwrap();
    let (user_heap, user_heap_end) = paging::get_user_heap(executable.page_table_phys().as_u64()).unwrap();

    let new_thread = {
        let thread_id = new_thread_id();
        let kernel_stack = Vec::with_capacity(KERNEL_STACK_SIZE as usize);
        let kernel_stack_end =
            (SnVirtAddr::from_ptr(kernel_stack.as_ptr()) + KERNEL_STACK_SIZE).as_u64();

        let context = kernel_stack_end - INTERRUPT_CONTEXT_SIZE as u64;
        // The 4096 (1 page) offset is a guard page
        crate::hal::interface::interrupt::without_interrupts(|| {
            crate::hal::interface::paging::with_page_table(executable.page_table_phys(), || {
                crate::hal::interface::paging::map_user_allocate_mem(
                    SnVirtAddr::new(user_stack),
                    SnVirtAddr::new(user_stack_end),
                );
                crate::hal::interface::paging::map_user_allocate_mem(
                    SnVirtAddr::new(user_heap),
                    SnVirtAddr::new(user_heap_end),
                );
            })
        });

        Box::new(Thread {
            id: new_thread_id(),
            process: Arc::new(Process {
                id: new_process_id(),
                page_table_phys_addr: executable.page_table_phys().as_u64(),
            }
            ),
            kernel_stack,
            kernel_stack_end,
            user_stack_end,
            context,
            page_table_addr: executable.page_table_phys().as_u64(),
        })
    };

    unsafe {
        crate::hal::interface::cpu::set_context(
            new_thread.context,
            executable.entry_point().as_u64(),
            new_thread.user_stack_end,
            true,
        );

        // set registers for heap alloc

        let context = &mut *(new_thread.context as *mut SnCpuContext) ;
        context.set_heap_addrs(user_heap as usize, user_heap_end as usize);
    };

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
        // // Save the location of the Context struct
        thread.context = context_addr as u64;
        // Save the page table. This is to enable context
        // switching during functions which manipulate page tables
        // for example new_user_thread
        thread.page_table_addr = crate::hal::interface::paging::get_current_page_table_phys_addr();

        // Put to the back of the queue
        running_queue.push_back(thread);
    }

    *current_thread = running_queue.pop_front();
    match current_thread.as_ref() {
        Some(thread) => {
            // Set the kernel stack for the next interrupt
            crate::hal::interface::interrupt::set_interrupt_stack_table(
                InterruptStackIndex::Timer as usize,
                SnVirtAddr::new(thread.kernel_stack_end),
            );

            if thread.page_table_addr != 0 {
                // Change page table
                // Note: zero for kernel thread
                paging::switch_page_table(SnPhysAddr::new(thread.page_table_addr));
            }

            // Point the stack to the new context
            thread.context as usize
        }
        None => 0, // Timer handler won't modify stack
    }
}

pub fn fork_current_thread(current_context: &mut SnCpuContext) {
    if let Some(current_thread) = CURRENT_THREAD.read().as_ref() {
        printk!(
            "process: forking thread {:x}",
            current_context.instruction_pointer(),
        );

        let page_table_phys_addr =
            crate::hal::interface::paging::get_current_page_table_phys_addr();

        let new_thread = {
            let thread_id = new_thread_id();
            let kernel_stack = Vec::with_capacity(KERNEL_STACK_SIZE as usize);
            let kernel_stack_end =
                (SnVirtAddr::from_ptr(kernel_stack.as_ptr()) + KERNEL_STACK_SIZE).as_u64();

            let context = kernel_stack_end - INTERRUPT_CONTEXT_SIZE as u64;
            // The 4096 (1 page) offset is a guard page
            
        let (user_stack, user_stack_end) = paging::get_user_thread_stack(page_table_phys_addr).unwrap();

            crate::hal::interface::interrupt::without_interrupts(|| {
                crate::hal::interface::paging::with_page_table(
                    SnPhysAddr::new(page_table_phys_addr),
                    || {
                        crate::hal::interface::paging::map_user_allocate_mem(
                            SnVirtAddr::new(user_stack),
                            SnVirtAddr::new(user_stack_end),
                        );
                    },
                )
            });

            Box::new(Thread {
                id: new_thread_id(),
                process: current_thread.process.clone(),
                kernel_stack,
                kernel_stack_end,
                user_stack_end,
                context,
                page_table_addr: page_table_phys_addr,
            })
        };

        unsafe {
            crate::hal::interface::cpu::set_context(
                new_thread.context,
                current_context.instruction_pointer() as u64,
                new_thread.user_stack_end,
                true,
            )
        };

        let new_context = unsafe { &mut *(new_thread.context as *mut SnCpuContext) };
        *new_context = current_context.clone(); // Copy of caller

        new_context.set_ret_val_1(0); // No error
        new_context.set_arg_val_1(0); // Indicates that this is the new thread
        current_context.set_ret_val_1(0); // Also success
        current_context.set_arg_val_1(new_thread.id as usize);

        crate::hal::interface::interrupt::without_interrupts(|| {
            RUNNING_QUEUE.get().unwrap().write().push_back(new_thread);
        });
    } else {
        current_context.set_ret_val_1(1);
    }
}

pub fn exit_current_thread(_current_context: &mut SnCpuContext) {
    {
        let mut current_thread = CURRENT_THREAD.write();

        if let Some(_thread) = current_thread.take() {
            // Drop thread, freeing stacks. If this is the last thread
            // in this process, memory and page tables will be freed
            // in the Process drop() function
        }
    }
    // Can't return from this syscall, so this thread now waits for a
    // timer interrupt to switch context.
    unsafe {
        asm!("sti", "2:", "hlt", "jmp 2b");
    }
}

pub fn init() {
    printk!("process: setting the scheduler");
    SCHEDULE.init_once(move || schedule_next);
}
