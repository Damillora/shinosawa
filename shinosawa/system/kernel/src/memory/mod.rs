use core::fmt::{self, Debug};

use crate::{hal, printk};

pub mod alloc;

pub trait SnAddr: Debug {
    fn as_u64(&self) -> u64;

    fn new(addr: u64) -> Self;
}
pub struct SnVirtAddr(u64);

impl SnAddr for SnVirtAddr {
    fn as_u64(&self) -> u64 {
        self.0
    }

    fn new(addr: u64) -> Self {
        Self(addr)
    }
}

impl Debug for SnVirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct SnPhysAddr(u64);

impl SnAddr for SnPhysAddr {
    fn as_u64(&self) -> u64 {
        return self.0;
    }

    fn new(addr: u64) -> Self {
        Self(addr)
    }
}

impl Debug for SnPhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


#[test_case]
fn memory_map() {

    let addresses = [
        hal::interface::paging::PHYSICAL_MEM_OFFSET.as_u64(),
    ];

    for &address in &addresses {
        let virt = SnVirtAddr::new(address);
        let virt_addr = virt.as_u64();
        let phys = unsafe { crate::hal::interface::paging::translate_addr(virt) };
        printk!("translate: {:?} -> {:?}", virt_addr, phys);
    }
}