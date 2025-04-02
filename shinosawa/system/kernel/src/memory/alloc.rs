// in src/allocator.rs

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB

pub const ACPI_START: usize = 0x_3333_0000_0000;

use linked_list_allocator::LockedHeap;

use crate::printk;

use super::SnVirtAddr;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Create heap for allocator
pub fn init()  {
    printk!("memory: initializing allocator");
    let start_addr = SnVirtAddr::new(HEAP_START as u64);
    let end_addr = start_addr + HEAP_SIZE as u64 - 1u64;

    crate::hal::interface::paging::map_new_memory(start_addr, end_addr);

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
}