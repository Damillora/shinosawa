use acpi::{AcpiHandler, AcpiTables};

use crate::{hal::x86_64::acpi::SnAcpiHandler, printk, println};

pub fn init() {
    printk!("acpi: initializing");
    if let Some(req) = crate::limine::RSDP_REQUEST.get_response() {
        let addr = req.address();
        let handler = SnAcpiHandler::new();

        let acpi_table = unsafe { AcpiTables::from_rsdp(handler, addr) };

        for acpi in acpi_table.unwrap().dsdt() {
            println!("DSDT: {}", acpi.address);
        }
    }
}
