use core::fmt::{self, Debug};

use ::alloc::{boxed::Box,  vec, rc::Rc, vec::Vec};

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
fn test_memory_alloc() {
    
    // allocate a number on the heap
    let heap_value = Box::new(41);
    printk!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    printk!("vec at {:p}", vec.as_slice());

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    printk!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    printk!("reference count is {} now", Rc::strong_count(&cloned_reference));
}