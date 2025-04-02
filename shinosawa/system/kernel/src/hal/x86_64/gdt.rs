use conquer_once::spin::OnceCell;
use spin::Mutex;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::{VirtAddr, structures::gdt::SegmentSelector};

use crate::printk;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const GENERAL_PROTECTION_FAULT_IST_INDEX: u16 = 0;
pub const TIMER_INTERRUPT_INDEX: u16 = 1; // New

static TSS: OnceCell<Mutex<TaskStateSegment>> = OnceCell::uninit();
static GDT: OnceCell<(GlobalDescriptorTable, Selectors)> = OnceCell::uninit();

unsafe fn tss_reference() -> &'static TaskStateSegment {
    let tss_ptr = &*TSS.get().unwrap().lock() as *const TaskStateSegment;
    unsafe { &*tss_ptr }
}

pub fn set_interrupt_stack_table(index: usize, stack_end: VirtAddr) {
    TSS.get().unwrap().lock().interrupt_stack_table[index] = stack_end;
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    data_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::segmentation::{CS, SS, Segment};
    use x86_64::instructions::tables::load_tss;
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

        tss.interrupt_stack_table[TIMER_INTERRUPT_INDEX as usize] =
            tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize]; // New

        Mutex::new(tss)
    });

    printk!("x86_64: initializing GDT");
    GDT.init_once(move || {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(unsafe { tss_reference() }));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
                data_selector,
            },
        )
    });

    GDT.get().unwrap().0.load();
    unsafe {
        SS::set_reg(SegmentSelector { 0: 0 });
        CS::set_reg(GDT.get().unwrap().1.code_selector);
        load_tss(GDT.get().unwrap().1.tss_selector);
    }
}

pub fn get_kernel_segments() -> (SegmentSelector, SegmentSelector) {
    (
        GDT.get().unwrap().1.code_selector,
        GDT.get().unwrap().1.data_selector,
    )
}
