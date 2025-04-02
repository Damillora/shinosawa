use acpi::InterruptModel;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use x2apic::lapic::{LocalApic, LocalApicBuilder};
use x86_64::PhysAddr;

use crate::{acpi::HARDWARE_INFO, hal::x86_64::{interrupt::InterruptIndex, paging}, printk};
use core::ops::{Deref, DerefMut};

// FIXME: this is not sound
pub struct UnsafeLocalApic(pub LocalApic);

unsafe impl Send for UnsafeLocalApic {}
unsafe impl Sync for UnsafeLocalApic {}

impl Deref for UnsafeLocalApic {
    type Target = LocalApic;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UnsafeLocalApic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub static LOCAL_APIC: OnceCell<Mutex<UnsafeLocalApic>> = OnceCell::uninit();


pub fn init() {
    printk!("x86_64::apic: initializing");

    let hw_info = HARDWARE_INFO.get().unwrap();
    if let InterruptModel::Apic(apic) = &hw_info.interrupt_model {
        printk!("x86_64::apic: this system has APIC");
        let apic_physical_address: u64 = apic.local_apic_address ;
        let apic_virtual_address: u64 = paging::phys_to_virt_addr(PhysAddr::new(apic_physical_address)).as_u64();
    
        let lapic = LOCAL_APIC.get_or_init(move || 
            Mutex::new(
                UnsafeLocalApic({
                    LocalApicBuilder::new()
                        .timer_vector(InterruptIndex::ApicTimer.as_usize())
                        .error_vector(InterruptIndex::ApicError.as_usize())
                        .spurious_vector(InterruptIndex::ApicSpurious.as_usize())
                        .set_xapic_base(apic_virtual_address)
                        .build()
                        .unwrap_or_else(|err| panic!("x86_64::apic: {}", err))
                })
            )
        );
        printk!("x86_64::apic: local APIC yeeee");
        unsafe {
            let mut lapic = lapic.lock();
            lapic.enable();
        }
    } else {
        printk!("x86_64::apic: this system does not use APIC, apparently");
    }

}