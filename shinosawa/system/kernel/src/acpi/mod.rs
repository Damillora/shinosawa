use core::ptr::NonNull;

use acpi::{fadt::Fadt, madt::Madt, AcpiHandler, AcpiTables, InterruptModel, PhysicalMapping};
use conquer_once::spin::OnceCell;

use crate::{
    hal,
    memory::{
        SnPhysAddr, SnVirtAddr,
        alloc::ACPI_START,
    },
    printk,
};

pub static HARDWARE_INFO: OnceCell<SnHardwareInfo> = OnceCell::uninit();

#[derive(Clone)]
pub struct SnHardwareInfo<'a> {
    pub interrupt_model: InterruptModel<'a, alloc::alloc::Global>,
    pub processor_info: Option<acpi::platform::ProcessorInfo<'a, alloc::alloc::Global>>
}

pub fn init() {
    printk!("acpi: initializing");
    if let Some(req) = crate::limine::RSDP_REQUEST.get_response() {
        let addr = req.address();
        let handler = SnAcpiHandler::new();

        let acpi_result = unsafe { AcpiTables::from_rsdp(handler, addr) };
        let acpi_table = acpi_result.as_ref().unwrap();

        if let Ok(acpi) = acpi_table.dsdt() {
            printk!("acpi: DSDT: {:#x}", acpi.address);
        }
        for acpi in acpi_table.ssdts() {
            printk!("acpi: SSDT: {:#x}", acpi.address);
        }

        let madt = acpi_table.find_table::<Madt>().unwrap();
        printk!("acpi: MADT: {:#x}", madt.physical_start());

        let fadt = acpi_table.find_table::<Fadt>().unwrap();
        printk!("acpi: FADT: {:#x}", fadt.physical_start());

        let (interrupt_model, processor_info) = madt
            .get()
            .parse_interrupt_model_in(alloc::alloc::Global)
            .unwrap();

        HARDWARE_INFO.init_once(move || SnHardwareInfo {
            interrupt_model: interrupt_model,
            processor_info: processor_info,
        });
    }
}

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
