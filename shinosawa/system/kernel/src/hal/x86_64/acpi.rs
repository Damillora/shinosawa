use core::ptr::NonNull;

use acpi::{AcpiHandler, PhysicalMapping};
use x86_64::{
    structures::paging::{page_table, Mapper, Page, PhysFrame, Size4KiB}, PhysAddr, VirtAddr
};

use crate::{hal::x86_64::paging::init_page_table, limine::MEMORY_MAP_REQUEST, memory::alloc::ACPI_START, println};

use super::paging::MEMORY_INFO;

#[derive(Clone, Copy)]
pub struct SnAcpiHandler {
}
impl SnAcpiHandler {
    pub fn new() -> SnAcpiHandler {
        SnAcpiHandler { }
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
        let memory_info = unsafe {MEMORY_INFO.as_mut().unwrap()};

        let mut page_table = init_page_table(memory_info.physical_memory_offset);

        use x86_64::structures::paging::PageTableFlags as Flags;
        
        // map an unused page
        let virt_addr = VirtAddr::new(ACPI_START as u64);
        let page: Page<Size4KiB> = Page::containing_address(virt_addr);

        println!("x86_64::acpi: map {} {}", physical_address, size);
        
        let frame = PhysFrame::containing_address(PhysAddr::new(physical_address as u64));
        let flags = Flags::PRESENT | Flags::WRITABLE;

        let map_to_result = unsafe {
            // FIXME: this is not safe, we do it only for testing
            page_table
                .map_to(page, frame, flags, &mut memory_info.frame_allocator)
        };
        map_to_result.expect("map_to failed").flush();

        return unsafe {
            PhysicalMapping::new(
                physical_address,
                NonNull::new(virt_addr.as_mut_ptr()).expect("cannot create NonNull"),
                size,
                size,
                *self,
            )
        };
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        // FIXME: static mutable
        #[allow(static_mut_refs)]
        let memory_info = unsafe {MEMORY_INFO.as_mut().unwrap()};

        let mut page_table = unsafe { init_page_table(memory_info.physical_memory_offset) };

        let page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(region.virtual_start().as_ptr() as u64));

        page_table
            .unmap(page).expect("cannot unmap");
    }
}
