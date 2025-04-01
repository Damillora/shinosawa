// in src/memory.rs

use limine::memory_map::{self, EntryType};
use x86_64::{structures::paging::{OffsetPageTable, PageTable, Translate, Page, PhysFrame, Mapper, Size4KiB, FrameAllocator}, PhysAddr, VirtAddr};

use crate::{limine::MEMORY_MAP_REQUEST, memory::{SnAddr, SnPhysAddr, SnVirtAddr}, printk};

use lazy_static::lazy_static;

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

lazy_static! {
    pub static ref PHYSICAL_MEM_OFFSET: SnVirtAddr = {
        if let Some(res) = crate::limine::HHDM_REQUEST.get_response() {
            return SnVirtAddr::new(res.offset());
        } else {
            panic!("cannot get HHDM");
        }
    };
}


unsafe fn init_page_table() -> OffsetPageTable<'static> {
    printk!("x86_64: initializing page table translation");
    let physical_memory_offset = VirtAddr::new(PHYSICAL_MEM_OFFSET.as_u64());
    
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

pub fn init() {
    printk!("x86_64: checking mapping");
    if let Some(resp) = MEMORY_MAP_REQUEST.get_response() {
        let mut frame_allocator = unsafe {
            SnLimineFrameAllocator::init(resp.entries())
        };

        let mut page_table = unsafe { init_page_table() };

        // map an unused page   
        let page = Page::containing_address(VirtAddr::new(0));
        create_example_mapping(page, &mut page_table, &mut frame_allocator);
    
        // write the string `New!` to the screen through the new mapping
        let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
        unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};
    }
}

pub struct SnFrameMapper {
    page_table: &'static mut OffsetPageTable<'static>,
    frame_allocator: &'static mut SnLimineFrameAllocator,
}

/// Translates the given virtual address to the mapped physical address, or
/// `None` if the address is not mapped.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`.
pub unsafe fn translate_addr(addr: SnVirtAddr)
    -> Option<SnPhysAddr>
{
    let page_table = unsafe { init_page_table() };

    let virt  = VirtAddr::new(addr.as_u64());
    let translated = page_table.translate_addr(virt);

    match translated {
        Some(translated) => Some(SnPhysAddr::new(translated.as_u64())),
        None => None
    }
}

/// Creates an example mapping for the given page to frame `0xb8000`.
fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe {
        // FIXME: this is not safe, we do it only for testing
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("map_to failed").flush();
}

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
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
    pub unsafe fn init(memory_map: &'static [&'static memory_map::Entry]) -> Self {
        SnLimineFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.entry_type == EntryType::USABLE);
        // map each region to its address range
        let addr_ranges = usable_regions
            .map(|r| r.base..(r.base + r.length));
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