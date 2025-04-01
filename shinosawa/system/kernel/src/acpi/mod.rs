use acpi::{ AcpiTables};

use crate::{hal::x86_64::acpi::SnAcpiHandler, printk, println};

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
