// Allow mutable static, since any other way would lock things up for now.
#![allow(static_mut_refs)]


use conquer_once::spin::OnceCell;
use spin::RwLock;
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3Flags,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB, mapper::MapToError, page,
    },
};

use crate::{
    limine::MEMORY_MAP_REQUEST,
    memory::{SnPhysAddr, SnVirtAddr},
    printk,
};

use super::frame_alloc::SnLimineFrameAllocator;

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

    unsafe { get_page_table_from_address(physical_memory_offset, phys.as_u64()) }
}

pub fn get_current_page_table_phys_addr() -> u64 {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    level_4_table_frame.start_address().as_u64()
}

pub unsafe fn get_page_table_from_address(
    physical_memory_offset: VirtAddr,
    phys_address: u64,
) -> &'static mut PageTable {
    let virt = physical_memory_offset + phys_address;
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

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

#[allow(static_mut_refs)]
pub fn init() {
    if let Some(res) = crate::limine::HHDM_REQUEST.get_response() {
        let physical_memory_offset = VirtAddr::new(res.offset()); // Store boot_info for later calls

        if let Some(resp) = MEMORY_MAP_REQUEST.get_response() {
            let frame_allocator =
                unsafe { SnLimineFrameAllocator::init(resp.entries(), physical_memory_offset) };
            let kernel_l4_table = unsafe { active_level_4_table(physical_memory_offset) };

            unsafe {
                MEMORY_INFO = Some(MemoryInfo {
                    physical_memory_offset,
                    frame_allocator,
                    kernel_l4_table: kernel_l4_table,
                })
            };
        }
    } else {
        panic!("cannot get HHDM");
    }
}

pub fn phys_to_virt_addr(phys: PhysAddr) -> VirtAddr {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

    return memory_info.physical_memory_offset + phys.as_u64();
}

fn create_empty_pagetable() -> (*mut PageTable, u64) {
    // Need to borrow as mutable so that we can allocate new frames
    // and so modify the frame allocator
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

    // Get a frame to store the level 4 table
    let level_4_table_frame = memory_info.frame_allocator.allocate_frame().unwrap();
    let phys = level_4_table_frame.start_address(); // Physical address
    let virt = memory_info.physical_memory_offset + phys.as_u64(); // Kernel virtual address
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    // Clear all entries in the page table
    unsafe {
        (*page_table_ptr).zero();
    }

    (page_table_ptr, phys.as_u64())
}

pub fn create_new_user_pagetable() -> (SnVirtAddr, SnPhysAddr) {
    let (page_table_ptr, page_table_phys_addr) = create_empty_pagetable();
    let table = unsafe { &mut *page_table_ptr };

    printk!(
        "paging::user_pagetable: {:x} {:x}",
        page_table_ptr as u64,
        page_table_phys_addr
    );

    fn copy_pages_rec(
        physical_memory_offset: VirtAddr,
        from_table: &PageTable,
        to_table: &mut PageTable,
        level: u16,
    ) {
        for (i, entry) in from_table.iter().enumerate() {
            if !entry.is_unused() {
                if (level == 1) || entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                    // Maps a frame, not a page table
                    to_table[i].set_addr(entry.addr(), entry.flags());
                } else {
                    // Create a new table at level - 1
                    let (new_table_ptr, new_table_physaddr) = create_empty_pagetable();
                    let to_table_m1 = unsafe { &mut *new_table_ptr };

                    // Point the entry to the new table
                    to_table[i].set_addr(PhysAddr::new(new_table_physaddr), entry.flags());

                    // Get reference to the input level-1 table
                    let from_table_m1 = {
                        let virt = physical_memory_offset + entry.addr().as_u64();
                        unsafe { &*virt.as_ptr() }
                    };

                    // Copy level-1 entries
                    copy_pages_rec(
                        physical_memory_offset,
                        from_table_m1,
                        to_table_m1,
                        level - 1,
                    );
                }
            }
        }
    }

    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;
    let kernel_page_table = memory_info.kernel_l4_table.clone();

    copy_pages_rec(physical_memory_offset, &kernel_page_table, table, 4);

    (
        SnVirtAddr::from_ptr(page_table_ptr),
        SnPhysAddr::new(page_table_phys_addr),
    )
}

pub fn with_page_table<T: FnOnce()>(page_table_phys_addr: SnPhysAddr, func: T) {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();

    switch_page_table(page_table_phys_addr);

    func();

    switch_page_table(SnPhysAddr::new(phys.as_u64()));
}

pub fn switch_page_table(page_table_phys_addr: SnPhysAddr) {
    use x86_64::registers::control::Cr3;

    let frame =
        PhysFrame::from_start_address(PhysAddr::new(page_table_phys_addr.as_u64())).unwrap();
    unsafe { Cr3::write(frame, Cr3Flags::empty()) };
}

/// Map a single phys page
pub fn map_phys_page(phys_addr: SnPhysAddr, virt_addr: SnVirtAddr) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;

    let mut mapper: OffsetPageTable<'_> = unsafe { init_page_table(physical_memory_offset) };

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
    let page = Page::containing_address(virt);
    let frame = PhysFrame::containing_address(addr);

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
    unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };

    Ok(())
}

pub fn map_new_memory(start_addr: SnVirtAddr, end_addr: SnVirtAddr) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;

    let mut mapper: OffsetPageTable<'_> = unsafe { init_page_table(physical_memory_offset) };

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
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;

    let mut mapper: OffsetPageTable<'_> = unsafe { init_page_table(physical_memory_offset) };

    let start_addr_x86 = VirtAddr::new(start_addr.as_u64());
    let end_addr_x86 = VirtAddr::new(end_addr.as_u64());
    let phys_addr_start_x86 = PhysAddr::new(phys_addr_start.as_u64());

    unsafe {
        map_phys_memory_inner(
            &mut mapper,
            &mut memory_info.frame_allocator,
            start_addr_x86,
            end_addr_x86,
            phys_addr_start_x86,
            size,
        )
        .expect("cannot map memory")
    }
}

unsafe fn map_phys_memory_inner(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_addr: VirtAddr,
    end_addr: VirtAddr,
    phys_addr_start: PhysAddr,
    size: usize,
) -> Result<u64, MapToError<Size4KiB>> {
    let page_range = {
        let start = start_addr.clone();
        let end = end_addr.clone();
        let start_page = Page::containing_address(start);
        let end_page = Page::containing_address(end);
        Page::range_inclusive(start_page, end_page)
    };

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

    let mut counter = 0;
    for page in page_range {
        if let Err(_) = mapper.translate_page(page) {
            let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(PhysAddr::new(
                phys_addr_start.as_u64() as u64 + counter,
            ));

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

/// Maps user accessible memory
pub fn map_user_memory(start_addr: SnVirtAddr, end_addr: SnVirtAddr) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;

    let mut mapper: OffsetPageTable<'_> = unsafe { init_page_table(physical_memory_offset) };
    let start_addr_x86 = VirtAddr::new(start_addr.as_u64());
    let end_addr_x86 = VirtAddr::new(end_addr.as_u64());

    map_user_memory_inner(
        &mut mapper,
        &mut memory_info.frame_allocator,
        start_addr_x86,
        end_addr_x86,
    )
    .expect("cannot map memory")
}

/// Create heap
fn map_user_memory_inner(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_addr: VirtAddr,
    end_addr: VirtAddr,
) -> Result<(), MapToError<Size4KiB>> {
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
        let flags =
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    Ok(())
}

pub fn unmap_memory(start_addr: SnVirtAddr, end_addr: SnVirtAddr) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

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
