// in src/allocator.rs


use crate::printk;

use super::{linked_list::LinkedListAllocator, Locked, SnVirtAddr};

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 10 * 1024 * 1024; // 10 KiB

pub const ACPI_START: usize = 0x_4444_0000_0000;

#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> =
    Locked::new(LinkedListAllocator::new());

/// Create heap for allocator
pub fn init()  {
    let start_addr = SnVirtAddr::new(HEAP_START as u64);
    let end_addr = start_addr + HEAP_SIZE as u64 - 1u64;

    crate::hal::interface::paging::map_new_memory(start_addr, end_addr);

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
}