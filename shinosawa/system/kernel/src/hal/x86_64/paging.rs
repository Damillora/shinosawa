// Allow mutable static, since any other way would lock things up for now.
#![allow(static_mut_refs)]

use core::ops::Range;

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
    memory::{SnPhysAddr, SnVirtAddr, USER_HEAP_SIZE, USER_STACK_SIZE},
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

    // Investigate why removing this causes physical address errors
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
pub fn map_user_executable_memory(start_addr: SnVirtAddr, end_addr: SnVirtAddr) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;

    let mut mapper: OffsetPageTable<'_> = unsafe { init_page_table(physical_memory_offset) };
    let start_addr_x86 = VirtAddr::new(start_addr.as_u64());
    let end_addr_x86 = VirtAddr::new(end_addr.as_u64());
    printk!("x86_paging: {:X} {:X}", start_addr_x86, end_addr_x86);
    map_user_memory_inner(
        &mut mapper,
        &mut memory_info.frame_allocator,
        start_addr_x86,
        end_addr_x86,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
    )
    .expect("cannot map memory")
}

/// Create heap
fn map_user_memory_inner(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_addr: VirtAddr,
    end_addr: VirtAddr,
    page_table_flags: PageTableFlags,
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
        if mapper.translate_page(page).is_ok() {
            continue;
        }
        
        unsafe {
            mapper
                .map_to(page, frame, page_table_flags, frame_allocator)?
                .flush()
        };
    }

    Ok(())
}

/// Maps user accessible memory
pub fn map_user_allocate_mem(start_addr: SnVirtAddr, end_addr: SnVirtAddr) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;

    let mut mapper: OffsetPageTable<'_> = unsafe { init_page_table(physical_memory_offset) };
    let start_addr_x86 = VirtAddr::new(start_addr.as_u64());
    let end_addr_x86 = start_addr_x86 + 4095;
    let start_ro_addr_x86 = start_addr_x86 + 4096;
    let end_ro_addr_x86 = VirtAddr::new(end_addr.as_u64());

    map_user_memory_inner(
        &mut mapper,
        &mut memory_info.frame_allocator,
        start_addr_x86,
        end_addr_x86,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
    )
    .expect("cannot map memory");

    map_user_memory_inner(
        &mut mapper,
        &mut memory_info.frame_allocator,
        start_ro_addr_x86,
        end_ro_addr_x86,
        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE,
    )
    .expect("cannot map memory");
}

fn find_empty_stack_entry(level_4_table: *mut PageTable, idx_1: u64, idx_2: u64) -> *mut PageTable {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let mut table = unsafe { &mut *level_4_table };
    let thread_stack_page_index: [u64; 2] = [idx_1, idx_2];
    for index in thread_stack_page_index {
        let entry = &mut table[index as usize];
        if entry.is_unused() {
            let (new_table_ptr, new_table_physaddr) = create_empty_pagetable();
            entry.set_addr(
                PhysAddr::new(new_table_physaddr),
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            );
        }
        table = unsafe {
            &mut *(memory_info.physical_memory_offset + entry.addr().as_u64()).as_mut_ptr()
        };
    }

    table
}

/// Maps user accessible memory
pub fn get_user_thread_stack(page_table_phys_addr: u64) -> Result<(u64, u64), &'static str> {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;
    let mut table =
        unsafe { get_page_table_from_address(physical_memory_offset, page_table_phys_addr) };
    let mut thread_stack_index: [u64; 3] = [0, 0, 0];
    'all: for idx_1 in 3..6 {
        for idx_2 in 0..511 {
            let page_table = unsafe { &mut *(find_empty_stack_entry(table, idx_1, idx_2)) };
            for idx_3 in 0..512 {
                if page_table[idx_3].is_unused() {
                    table = unsafe { &mut *(page_table) };
                    thread_stack_index[0] = idx_1;
                    thread_stack_index[1] = idx_2;
                    thread_stack_index[2] = idx_3 as u64;
                    break 'all;
                }
            }
        }
    }
    if thread_stack_index[0] == 0 {
        return Err("All thread stack slots are full");
    }
    let slot_address: u64 = ((thread_stack_index[0] as u64) << 39)
        + ((thread_stack_index[1] as u64) << 30)
        + ((thread_stack_index[2] as u64) << 21);

    Ok((slot_address + 4096, slot_address + USER_STACK_SIZE))
}

/// Maps user accessible memory
pub fn get_user_heap(page_table_phys_addr: u64) -> Result<(u64, u64), &'static str> {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;
    let mut table =
        unsafe { get_page_table_from_address(physical_memory_offset, page_table_phys_addr) };
    let mut thread_stack_index: [u64; 3] = [0, 0, 0];
    'all: for idx_1 in 7..11 {
        for idx_2 in 0..511 {
            let page_table = unsafe { &mut *(find_empty_stack_entry(table, idx_1, idx_2)) };
            for idx_3 in 0..256 {
                if page_table[idx_3 * 2 + 1].is_unused() {
                    table = unsafe { &mut *(page_table) };
                    thread_stack_index[0] = idx_1;
                    thread_stack_index[1] = idx_2;
                    thread_stack_index[2] = idx_3 as u64;
                    break 'all;
                }
            }
        }
    }
    if thread_stack_index[0] == 0 {
        return Err("All thread heap slots are full");
    }
    let slot_address: u64 = ((thread_stack_index[0] as u64) << 39)
        + ((thread_stack_index[1] as u64) << 30)
        + ((thread_stack_index[2] as u64) << 21);

    Ok((slot_address + 4096, slot_address + USER_HEAP_SIZE))
}

/// Maps user accessible memory
pub fn map_missing_user_page(start_addr: SnVirtAddr) -> Result<(), MapToError<Size4KiB>> {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let physical_memory_offset = memory_info.physical_memory_offset;

    let mut mapper: OffsetPageTable<'_> = unsafe { init_page_table(physical_memory_offset) };
    let start_addr_x86 = VirtAddr::new(start_addr.as_u64());

    map_missing_user_page_inner(
        &mut mapper,
        &mut memory_info.frame_allocator,
        start_addr_x86,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
    )
}

/// Create heap
fn map_missing_user_page_inner(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    start_addr: VirtAddr,
    page_table_flags: PageTableFlags,
) -> Result<(), MapToError<Size4KiB>> {
    let page = Page::containing_address(start_addr);
    let frame = frame_allocator
        .allocate_frame()
        .ok_or(MapToError::FrameAllocationFailed)?;
    let _ = mapper.unmap(page);
    unsafe {
        mapper
            .map_to(page, frame, page_table_flags, frame_allocator)?
            .flush()
    };

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

fn active_level_1_table_containing(addr: VirtAddr) -> &'static mut PageTable {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let current_page_table = get_current_page_table_phys_addr();
    let mut table = unsafe {
        get_page_table_from_address(memory_info.physical_memory_offset, current_page_table)
    };

    for index in [addr.p4_index(), addr.p3_index(), addr.p2_index()] {
        let entry = &mut table[index];
        table = unsafe {
            &mut *(memory_info.physical_memory_offset + entry.addr().as_u64()).as_mut_ptr()
        };
    }

    table
}

pub fn free_user_stack(stack_end: SnVirtAddr) -> Result<(), &'static str> {
    return Ok(());

    let addr = VirtAddr::new((stack_end - 1u64).as_u64()); // Address in last page
    let table = active_level_1_table_containing(VirtAddr::new(stack_end.as_u64()));

    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

    let iend = usize::from(addr.p1_index());
    for index in ((iend - 6)..=iend).rev() {
        let entry = &mut table[index];

        // Only writable pages have unique frames
        if entry.flags().contains(PageTableFlags::WRITABLE) {
            // Free this frame
            memory_info
                .frame_allocator
                .deallocate_frame(entry.frame().unwrap());
        }
        entry.set_flags(PageTableFlags::empty());
    }

    Ok(())
}

pub fn free_user_pagetables(page_table_phys_addr: u64) {
    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

    fn free_pages_rec(
        physical_memory_offset: VirtAddr,
        frame_allocator: &mut SnLimineFrameAllocator,
        table_physaddr: PhysAddr,
        level: u16,
    ) {
        let table = unsafe {
            &mut *(physical_memory_offset + table_physaddr.as_u64()).as_mut_ptr() as &mut PageTable
        };
        for entry in table.iter() {
            if !entry.is_unused() {
                if (level == 1) || entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                    // Maps a frame, not a page table
                    if entry.flags().contains(PageTableFlags::USER_ACCESSIBLE) {
                        // A user frame => deallocate
                        frame_allocator.deallocate_frame(entry.frame().unwrap());
                    }
                } else {
                    // A page table
                    free_pages_rec(
                        physical_memory_offset,
                        frame_allocator,
                        entry.addr(),
                        level - 1,
                    );
                }
            }
        }
        // Free page table
        frame_allocator.deallocate_frame(PhysFrame::from_start_address(table_physaddr).unwrap());
    }

    let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };
    let kernel_table_phys_addr = (memory_info.kernel_l4_table as *mut PageTable as u64)
        - memory_info.physical_memory_offset.as_u64();

    with_page_table(SnPhysAddr::new(kernel_table_phys_addr), || {
        free_pages_rec(
            memory_info.physical_memory_offset,
            &mut memory_info.frame_allocator,
            PhysAddr::new(page_table_phys_addr),
            4,
        );
    });
}
