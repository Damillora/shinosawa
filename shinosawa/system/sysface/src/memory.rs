use linked_list_allocator::LockedHeap;

use crate::{linked_list::LinkedListAllocator, println, print, _print, Locked};


#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

pub fn init(heap_start: usize, heap_end: usize) {
    {
        let lock = ALLOCATOR.try_lock().expect("cannot lock alloc?!");
    }

    unsafe {
        ALLOCATOR.lock().init(heap_start, heap_end - heap_start);
    };
}