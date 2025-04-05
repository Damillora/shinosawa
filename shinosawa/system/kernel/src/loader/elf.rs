use object::{Object, ObjectSegment};

use crate::{
    hal::x86_64::paging::{self, switch_page_table, with_page_table},
    memory::{SnPhysAddr, SnVirtAddr},
    printk,
};

use super::SnExecutable;

#[derive(Clone)]
pub struct SnElfExecutable {
    entry_point: SnVirtAddr,
    user_page_table_virt_addr: SnVirtAddr,
    user_page_table_phys_addr: SnPhysAddr,
}

impl SnExecutable for SnElfExecutable {
    fn entry_point(&self) -> SnVirtAddr {
        self.entry_point
    }

    fn page_table_virt(&self) -> SnVirtAddr {
        self.user_page_table_virt_addr
        // todo!()
    }

    fn page_table_phys(&self) -> SnPhysAddr {
        self.user_page_table_phys_addr
        // todo!()
    }
}

pub fn load_elf(bin: &[u8]) -> Result<SnElfExecutable, &'static str> {
    // Check the header
    const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

    let (user_page_table_virt_addr, user_page_table_physaddr) = paging::create_new_user_pagetable();

    if bin[0..4] != ELF_MAGIC {
        return Err("Expected ELF binary");
    }
    // Use the object crate to parse the ELF file
    // https://crates.io/crates/object
    if let Ok(obj) = object::File::parse(bin) {
        let entry_point = obj.entry();
        printk!("loader::elf: Entry point: {:#016X}", entry_point);

        crate::hal::interface::interrupt::without_interrupts(|| {
            with_page_table(user_page_table_physaddr, || {
                for segment in obj.segments() {
                    printk!(
                        "loader::elf: Section {:?} : {:#016X}",
                        segment.name(),
                        segment.address()
                    );

                    let segment_address = segment.address() as u64;
                    
                    crate::hal::interface::paging::map_user_memory(
                        SnVirtAddr::new(segment_address),
                        SnVirtAddr::new(segment_address) + segment.size() as u64,
                    );

                    if let Ok(data) = segment.data() {
                        // Copy data
                        let dest_ptr = segment_address as *mut u8;
                        for (i, value) in data.iter().enumerate() {
                            unsafe {
                                let ptr = dest_ptr.add(i);
                                core::ptr::write(ptr, *value);
                            }
                        }
                    }
                }
            });
        });
        return Ok(SnElfExecutable {
            entry_point: SnVirtAddr::new(entry_point),
            user_page_table_virt_addr,
            user_page_table_phys_addr: user_page_table_physaddr,
        });
    }
    Err("Could not parse ELF")
}
