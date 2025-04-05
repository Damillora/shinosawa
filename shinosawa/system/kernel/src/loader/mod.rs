use crate::memory::{SnPhysAddr, SnVirtAddr};

/// ELF loader
pub mod elf;

pub trait SnExecutable {
    fn entry_point(&self) -> SnVirtAddr;
    fn page_table_virt(&self) -> SnVirtAddr;
    fn page_table_phys(&self) -> SnPhysAddr;
}