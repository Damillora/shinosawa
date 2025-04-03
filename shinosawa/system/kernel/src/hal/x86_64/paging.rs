// in src/memory.rs

use conquer_once::spin::OnceCell;
use limine::memory_map::{self, EntryType};
use spin::RwLock;
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB, mapper::MapToError,
    },
};

use crate::{
    limine::MEMORY_MAP_REQUEST,
    memory::{
        SnPhysAddr, SnVirtAddr,
    },
    printk, printk_s,
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

pub struct MemoryInfo {
    pub physical_memory_offset: VirtAddr,

    /// Allocate empty frames
    pub frame_allocator: SnLimineFrameAllocator,
    kernel_l4_table: &'static mut PageTable,
}

pub static MEMORY_INFO: OnceCell<RwLock<MemoryInfo>> = OnceCell::uninit();

pub unsafe fn init_page_table(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

pub fn init() {
    printk!("x86_64: initializing paging infrastructure");
    if let Some(res) = crate::limine::HHDM_REQUEST.get_response() {
        let physical_memory_offset = VirtAddr::new(res.offset()); // Store boot_info for later calls

        if let Some(resp) = MEMORY_MAP_REQUEST.get_response() {
            let frame_allocator = unsafe { SnLimineFrameAllocator::init(resp.entries()) };

            MEMORY_INFO.init_once(move || RwLock::new( MemoryInfo {
                physical_memory_offset,
                frame_allocator,
                kernel_l4_table: unsafe { active_level_4_table(physical_memory_offset) },

            }));
        }
    } else {
        panic!("cannot get HHDM");
    }
}

/// Map a single phys page
pub fn map_phys_page(
    phys_addr: SnPhysAddr,
    virt_addr: SnVirtAddr,
) {
    let mut memory_info = MEMORY_INFO.get().unwrap().write();

    let mut mapper: OffsetPageTable<'_> =
        unsafe { init_page_table(memory_info.physical_memory_offset) };

    let phys_addr_x86 = PhysAddr::new(phys_addr.as_u64());
    let virt_addr_x86 = VirtAddr::new(virt_addr.as_u64());

    map_phys_page_inner(
        &mut mapper,
        &mut memory_info.frame_allocator,
        phys_addr_x86,
        virt_addr_x86,
    )
    .expect("cannot map memory")
}

fn map_phys_page_inner(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    addr: PhysAddr,
    virt: VirtAddr,
) -> Result<(), MapToError<Size4KiB>> {
    printk_s!(
        "x86_64::paging: page {:x} {:x}",
        addr.as_u64(),
        virt.as_u64(),
    );
    
    let page = Page::containing_address(virt);
    let frame = PhysFrame::containing_address(addr);

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
    unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    
    Ok(())
}

pub fn map_new_memory(
    start_addr: SnVirtAddr,
    end_addr: SnVirtAddr,
) {
    let mut memory_info = MEMORY_INFO.get().unwrap().write();

    let mut mapper: OffsetPageTable<'_> =
        unsafe { init_page_table(memory_info.physical_memory_offset) };

    let start_addr_x86 = VirtAddr::new(start_addr.as_u64());
    let end_addr_x86 = VirtAddr::new(end_addr.as_u64());

    map_new_memory_inner(
        &mut mapper,
        &mut memory_info.frame_allocator,
        start_addr_x86,
        end_addr_x86,
    )
    .expect("cannot map memory")
}

/// Create heap
fn map_new_memory_inner(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_addr: VirtAddr,
    end_addr: VirtAddr,
) -> Result<(), MapToError<Size4KiB>> {
    printk_s!(
        "x86_64::paging: new_map {:x}-{:x}",
        start_addr.as_u64(),
        end_addr.as_u64()
    );
    let page_range = {
        let heap_start = start_addr.clone();
        let heap_end = end_addr.clone();
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    Ok(())
}

/// Maps physical memory 
/// This one skips through mapped pages, so mark this as unsafe
pub unsafe fn map_phys_memory(
    start_addr: SnVirtAddr,
    end_addr: SnVirtAddr,
    phys_addr_start: SnPhysAddr,
    size: usize,
) -> u64 {
    let mut memory_info = MEMORY_INFO.get().unwrap().write();


    let mut mapper: OffsetPageTable<'_> =
        unsafe { init_page_table(memory_info.physical_memory_offset) };

    let start_addr_x86 = VirtAddr::new(start_addr.as_u64());
    let end_addr_x86 = VirtAddr::new(end_addr.as_u64());
    let phys_addr_start_x86 = PhysAddr::new(phys_addr_start.as_u64());

    unsafe { map_phys_memory_inner(
        &mut mapper,
        &mut memory_info.frame_allocator,
        start_addr_x86,
        end_addr_x86,
        phys_addr_start_x86,
        size,
    )
    .expect("cannot map memory") }
}

unsafe fn map_phys_memory_inner(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_addr: VirtAddr,
    end_addr: VirtAddr,
    phys_addr_start: PhysAddr,
    size: usize,
) -> Result<u64, MapToError<Size4KiB>> {
    printk_s!(
        "x86_64::paging: phys_map {:x} size {:#} to {:x}-{:x}",
        phys_addr_start.as_u64(),
        size,
        start_addr.as_u64(),
        end_addr.as_u64()
    );
    let page_range = {
        let start = start_addr.clone();
        let end = end_addr.clone();
        let start_page = Page::containing_address(start);
        let end_page = Page::containing_address(end);
        Page::range_inclusive(start_page, end_page)
    };

    let flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

    let mut counter = 0;
    for page in page_range {
        if let Err(_) = mapper.translate_page(page) {
            let frame: PhysFrame<Size4KiB> =
                PhysFrame::containing_address(PhysAddr::new(phys_addr_start.as_u64() as u64 + counter));
    
            let map_to_result = unsafe {
                // FIXME: this is not safe, we do it only for testing
                mapper.map_to(page, frame, flags, frame_allocator) 
            };
            map_to_result.expect("map_to failed").flush();
        }

        counter += page.size();
    }

    Ok(counter)
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
    pub unsafe fn init(
        memory_map: &'static [&'static memory_map::Entry],
    ) -> SnLimineFrameAllocator {
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

pub fn unmap_memory(
    start_addr: SnVirtAddr,
    end_addr: SnVirtAddr,
) {
    let memory_info = MEMORY_INFO.get().unwrap().write();

    let mut mapper = unsafe { init_page_table(memory_info.physical_memory_offset) };
    
    let start_addr_x86 = VirtAddr::new(start_addr.as_u64());
    let end_addr_x86 = VirtAddr::new(end_addr.as_u64());
    unmap_memory_inner(&mut mapper, start_addr_x86, end_addr_x86);
}

fn unmap_memory_inner(
    mapper: &mut impl Mapper<Size4KiB>,
    start_addr: VirtAddr,
    end_addr: VirtAddr,
) {
    printk_s!(
        "x86_64::paging: unmap {:x}-{:x}",
        start_addr.as_u64(),
        end_addr.as_u64()
    );

    let page_range = {
        let start = start_addr.clone();
        let end = end_addr.clone();
        let start_page = Page::containing_address(start);
        let end_page = Page::containing_address(end);
        Page::range_inclusive(start_page, end_page)
    };

    for page in page_range {
        mapper.unmap(page).expect("cannot unmap").1.flush();
    }

}

pub fn phys_to_virt_addr(phys: PhysAddr) -> VirtAddr {
    let memory_info = MEMORY_INFO.get().unwrap().read();

    return memory_info.physical_memory_offset + phys.as_u64();
}