use object::{Object, ObjectSegment};

use crate::{memory::SnVirtAddr, printk};

pub fn load_elf(bin: &[u8]) -> Result<SnVirtAddr, &'static str> {
    // Check the header
    const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

    if bin[0..4] != ELF_MAGIC {
        return Err("Expected ELF binary");
    }
    // Use the object crate to parse the ELF file
    // https://crates.io/crates/object
    if let Ok(obj) = object::File::parse(bin) {
        let entry_point = obj.entry();
        printk!("loader::elf: Entry point: {:#016X}", entry_point);

        for segment in obj.segments() {
            printk!("loader::elf: Section {:?} : {:#016X}", segment.name(), segment.address());

            let segment_address = segment.address() as u64;

            crate::hal::interface::paging::map_user_memory(SnVirtAddr::new(segment_address), SnVirtAddr::new(segment_address) + segment.size() as u64 );

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
        return Ok(SnVirtAddr::new(entry_point));
    }
    Err("Could not parse ELF")
}