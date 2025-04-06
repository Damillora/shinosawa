use spin::RwLock;
use x86_64::structures::paging::page;

use crate::{hal::x86_64::paging, printk};

pub struct Process {
    pub id: u64,
    pub page_table_phys_addr: u64,
}
impl Drop for Process {
    fn drop(&mut self) {
        printk!("process::process: dropping process {}", self.id);
        if self.page_table_phys_addr != 0 {
            paging::free_user_pagetables(self.page_table_phys_addr);
        }
    }
}
