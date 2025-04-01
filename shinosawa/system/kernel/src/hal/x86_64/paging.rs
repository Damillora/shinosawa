// in src/memory.rs

use limine::memory_map::{self, EntryType};
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB
    }, PhysAddr, VirtAddr
};

use crate::{
    limine::MEMORY_MAP_REQUEST,
    memory::alloc::{HEAP_SIZE, HEAP_START},
    printk,
};

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub struct MemoryInfo {
    pub physical_memory_offset: VirtAddr,

    /// Allocate empty frames
    pub frame_allocator: SnLimineFrameAllocator,
    kernel_l4_table: &'static mut PageTable,

}

pub static mut MEMORY_INFO: Option<MemoryInfo> = None;

pub unsafe fn init_page_table(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

pub fn init() {
    if let Some(res) = crate::limine::HHDM_REQUEST.get_response() {
        let physical_memory_offset = VirtAddr::new(res.offset());// Store boot_info for later calls
        
        if let Some(resp) = MEMORY_MAP_REQUEST.get_response() {
            let frame_allocator = unsafe { SnLimineFrameAllocator::init(resp.entries()) };

            let mut page_table: OffsetPageTable<'_> = unsafe { init_page_table(physical_memory_offset) };

            unsafe { MEMORY_INFO = Some(MemoryInfo {
                physical_memory_offset,
                frame_allocator,
                kernel_l4_table: active_level_4_table(physical_memory_offset),
            }) };

            // FIXME: Static mutable here, must there be something better
            #[allow(static_mut_refs)]
            let memory_info = unsafe {MEMORY_INFO.as_mut().unwrap()};
            
            printk!("x86_64: initializing kernel heap");
            init_heap(&mut page_table, &mut memory_info.frame_allocator)
                .expect("heap initialization failed");

        }
        
    } else {
        panic!("cannot get HHDM");
    }

}

/// Create heap
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}

pub struct SnLimineFrameAllocator {
    memory_map: &'static [&'static memory_map::Entry],
    next: usize,
}

impl SnLimineFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static [&'static memory_map::Entry]) -> SnLimineFrameAllocator {
        SnLimineFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.entry_type == EntryType::USABLE);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.base..(r.base + r.length));
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for SnLimineFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
