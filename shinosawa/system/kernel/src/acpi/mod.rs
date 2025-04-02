use core::ptr::NonNull;

use acpi::{AcpiHandler, AcpiTables, PhysicalMapping};

use crate::{
    hal, memory::{alloc::ACPI_START, SnPhysAddr, SnVirtAddr}, printk
};

#[derive(Clone, Copy)]
pub struct SnAcpiHandler;

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
        let start_addr = SnVirtAddr::new(ACPI_START as u64 + physical_address as u64);
        let end_addr = start_addr + size as u64;
        let phys_addr_start = SnPhysAddr::new(physical_address as u64);

        let mapped_size = unsafe {
            crate::hal::interface::paging::map_phys_memory(
                start_addr,
                end_addr,
                phys_addr_start,
                size,
            )
        };

        return unsafe {
            PhysicalMapping::new(
                usize::from(physical_address),
                NonNull::new(start_addr.as_mut_ptr()).unwrap(),
                size,
                mapped_size as usize,
                *self,
            )
        };
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        let start_addr = SnVirtAddr::new(region.virtual_start().as_ptr() as u64);
        let end_addr = start_addr + region.region_length() as u64;

        hal::interface::paging::unmap_memory(start_addr, end_addr);
    }
}

pub fn init() {
    printk!("acpi: initializing");
    if let Some(req) = crate::limine::RSDP_REQUEST.get_response() {
        let addr = req.address();
        let handler = SnAcpiHandler::new();

        let acpi_table = unsafe { AcpiTables::from_rsdp(handler, addr) };

        if let Ok(acpi) = acpi_table.as_ref().unwrap().dsdt() {
            printk!("acpi: DSDT: {:#x}", acpi.address);
        }
        for acpi in acpi_table.as_ref().unwrap().ssdts() {
            printk!("acpi: SSDT: {:#x}", acpi.address);
        }
    }
}
