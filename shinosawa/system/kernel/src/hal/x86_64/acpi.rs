use core::ptr::NonNull;

use acpi::{AcpiHandler, PhysicalMapping};
use embedded_graphics::prelude::Size;
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{
        Mapper, Page, PhysFrame, Size4KiB,
        page::{PageRange, PageRangeInclusive},
        page_table,
    },
};

use crate::{
    hal::x86_64::paging::{self, init_page_table}, limine::MEMORY_MAP_REQUEST, memory::alloc::ACPI_START, printk, println
};

use super::paging::MEMORY_INFO;

#[derive(Clone, Copy)]
pub struct SnAcpiHandler {}
impl SnAcpiHandler {
    pub fn new() -> SnAcpiHandler {
        SnAcpiHandler {}
    }
}
impl AcpiHandler for SnAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        // FIXME: static mutable
        #[allow(static_mut_refs)]
        let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

        let mut page_table = init_page_table(memory_info.physical_memory_offset);

        use x86_64::structures::paging::PageTableFlags as Flags;

        let start_addr =
            VirtAddr::new(ACPI_START as u64 + physical_address as u64);
        let end_addr = start_addr + size as u64;

        let page_range: PageRangeInclusive<Size4KiB> = {
            let start = start_addr.clone();
            let end = end_addr.clone();
            let start_page = Page::containing_address(start);
            let end_page = Page::containing_address(end);
            Page::range_inclusive(start_page, end_page)
        };
        let flags = Flags::PRESENT | Flags::WRITABLE | Flags::USER_ACCESSIBLE;
        let mut counter = physical_address as u64;
        for page in page_range {
            let frame: PhysFrame<Size4KiB> =
                PhysFrame::containing_address(PhysAddr::new(counter));

            let map_to_result = unsafe {
                // FIXME: this is not safe, we do it only for testing
                // Identity map
                unsafe { page_table.map_to(page, frame, flags, &mut memory_info.frame_allocator) }
            };
            map_to_result.expect("map_to failed").flush();

            counter += page.size();
        }

        printk!(
            "x86_64::acpi: map {:#x} {:#x} {}",
            physical_address,
            start_addr.as_u64(),
            size
        );

        return unsafe {
            PhysicalMapping::new(
                usize::from(physical_address),
                NonNull::new(start_addr.as_mut_ptr()).unwrap(),
                size,
                size,
                *self,
            )
        };
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        // FIXME: static mutable
        #[allow(static_mut_refs)]
        let memory_info = unsafe { MEMORY_INFO.as_mut().unwrap() };

        let mut page_table = unsafe { init_page_table(memory_info.physical_memory_offset) };

        let page: Page<Size4KiB> =
            Page::containing_address(VirtAddr::new(region.virtual_start().as_ptr() as u64));

        page_table.unmap(page).expect("cannot unmap");
    }
}
