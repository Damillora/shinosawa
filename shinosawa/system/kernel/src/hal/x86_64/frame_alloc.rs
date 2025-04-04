use core::arch::asm;

use limine::memory_map::{self, EntryType};
use x86_64::{structures::paging::{FrameAllocator, PhysFrame, Size4KiB}, PhysAddr, VirtAddr};

use crate::printk;

pub struct SnLimineFrameAllocator {
    memory_map: &'static [&'static memory_map::Entry],
    level_3_virt_addr: VirtAddr,
    level_2_virt_addr: VirtAddr,
    level_1_virt_addr: VirtAddr,
    frame_phys_addr: PhysAddr,
}

fn nonzero_bit_index(bitmap: u64) -> u64 {
    let index: u64;
    unsafe {
        asm!("bsf rax, rcx",
             in("rcx") bitmap,
             lateout("rax") index,
             options(pure, nomem, nostack));
    }
    index
}

impl SnLimineFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(
        memory_map: &'static [&'static memory_map::Entry],
        physical_memory_offset: VirtAddr
    ) -> SnLimineFrameAllocator {
        let mut usable_regions = memory_map
            .iter()
            .filter(|r| r.entry_type == EntryType::USABLE);

        let region = usable_regions.next().unwrap();

        let start_addr = region.base;
        let end_addr = region.base + region.length;
        let nframes = (end_addr - start_addr) / 4096 ;
        printk!("usable frame: {}", nframes);

        let level_3_virt_addr = physical_memory_offset + start_addr;
        let level_2_virt_addr = level_3_virt_addr + 8u64;
        let level_1_virt_addr = level_2_virt_addr + 8u64 * 8u64;
        
        let level_3_ptr = level_3_virt_addr.as_mut_ptr() as *mut u64;
        let level_2_ptr = level_2_virt_addr.as_mut_ptr() as *mut u64;
        let level_1_ptr = level_1_virt_addr.as_mut_ptr() as *mut u64;
        
        unsafe {
            *level_3_ptr = 0xFFFF_FFFF_FFFF_FFFF;
            *level_2_ptr = 0xFFFF_FFFF_FFFF_FFFF;
            *level_1_ptr = 0xFFFF_FFFF_FFFF_FFFE;
            
            for i in 0..63 {
                *(level_2_ptr.offset(i)) = 0xFFFF_FFFF_FFFF_FFFF;
                *level_1_ptr.offset(i * 64) = 0xFFFF_FFFF_FFFF_FFFE;

                for j in 1..63 {
                    *(level_1_ptr.offset(i * 64 + j)) = 0xFFFF_FFFF_FFFF_FFFF;
                }
            }
        }

        SnLimineFrameAllocator {
            level_3_virt_addr,
            level_2_virt_addr,
            level_1_virt_addr,
            frame_phys_addr: PhysAddr::new(start_addr),
            memory_map,
        }
    }
    
    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter().skip(1);
        let usable_regions = regions.filter(|r| r.entry_type == EntryType::USABLE);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.base..(r.base + r.length));
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    fn fetch_frame(&mut self) -> u64 {
        let l3_ptr = self.level_3_virt_addr.as_mut_ptr() as *mut u64;
        let mut l3_bitmap = unsafe{*l3_ptr};
        let l3_index = nonzero_bit_index(l3_bitmap);

        let l2_ptr = unsafe {
            (self.level_2_virt_addr.as_mut_ptr() as *mut u64)
            .offset(l3_index as isize)
        };
        let mut l2_bitmap = unsafe{*l2_ptr};
        let l2_index = nonzero_bit_index(l2_bitmap);

        let l1_ptr = unsafe{(self.level_1_virt_addr.as_mut_ptr() as *mut u64)
            .offset(l3_index as isize * 64 + l2_index as isize)};
        let mut l1_bitmap = unsafe{*l1_ptr};
        let l1_index = nonzero_bit_index(l1_bitmap);
        // printk_s!("l3: {:#} {:#} {:#}", l3_index, l2_index, l1_index);

        let frame_number =
            (l3_index as u64) * 64u64 * 64u64 +
            (l2_index as u64) * 64u64
            + (l1_index as u64);
        
        l1_bitmap ^= 1 << l1_index;
        unsafe{*l1_ptr = l1_bitmap;}

        if l1_bitmap == 0 {
            l2_bitmap ^= 1 << l2_index;
            unsafe{*l2_ptr = l2_bitmap;}

            if l2_bitmap == 0 {

                l3_bitmap ^= 1 << l3_index;
                unsafe{*l3_ptr = l3_bitmap;}
            }
        }

        frame_number
    }

    fn return_frame(&mut self, frame_number: u64) {
        // Calculate indices
        let l1_index = frame_number % (64 * 64);
        let l2_index = frame_number % 64;
        let l3_index = frame_number / 64;

        let l1_ptr = unsafe{(self.level_1_virt_addr.as_mut_ptr() as *mut u64)
            .offset(l3_index as isize * 64 +l2_index as isize)};
        unsafe{*l1_ptr |= 1 << l1_index;}

        // set level 2 bit
        let l2_ptr = self.level_2_virt_addr.as_mut_ptr() as *mut u64;
        unsafe{*l2_ptr |= 1 << l2_index};

        // set level 2 bit
        let l3_ptr = self.level_2_virt_addr.as_mut_ptr() as *mut u64;
        unsafe{*l2_ptr |= 1 << l3_index};
    }

    fn deallocate_frame(&mut self, frame: PhysFrame) {
        let frame_number = (frame.start_address() - self.frame_phys_addr) / 4096;
        self.return_frame(frame_number);
    }
}

unsafe impl FrameAllocator<Size4KiB> for SnLimineFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.fetch_frame();
        

        let frame = self.usable_frames().nth(frame as usize);

        frame
    }
    
}