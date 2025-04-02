
use conquer_once::spin::OnceCell;
use x86_64::{structures::gdt::SegmentSelector, VirtAddr};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};

use crate::printk;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

static TSS: OnceCell<TaskStateSegment> = OnceCell::uninit();
static GDT: OnceCell<(GlobalDescriptorTable, Selectors)> = OnceCell::uninit();

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS,SS, Segment};
    printk!("x86_64: initializing TSS");
    TSS.init_once(move || {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss
    });

    printk!("x86_64: initializing GDT");
    GDT.init_once(move ||  {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(TSS.get().unwrap()));
        (gdt, Selectors { code_selector, tss_selector })
    });

    GDT.get().unwrap().0.load();
    unsafe {
        SS::set_reg(SegmentSelector{0: 0});
        CS::set_reg(GDT.get().unwrap().1.code_selector);
        load_tss(GDT.get().unwrap().1.tss_selector);
    }
}