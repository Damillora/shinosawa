use core::{fmt::{self, Debug}, ops::{Add, Sub}};

pub mod alloc;

#[derive(Clone, Copy)]
pub struct SnVirtAddr(u64);

impl Add<u64> for SnVirtAddr {
    type Output = SnVirtAddr;

    fn add(self, rhs: u64) -> Self::Output {
        return SnVirtAddr::new(self.as_u64() + rhs)
    }
}

impl Sub<u64> for SnVirtAddr {
    type Output = SnVirtAddr;
    
    fn sub(self, rhs: u64) -> Self::Output {
        return SnVirtAddr::new(self.as_u64() + rhs)
    }

}

impl Debug for SnVirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SnVirtAddr {
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    #[cfg(target_pointer_width = "64")]
    #[inline]
    pub const fn as_ptr<T>(self) -> *const T {
        self.as_u64() as *const T
    }

    /// Converts the address to a mutable raw pointer.
    #[cfg(target_pointer_width = "64")]
    #[inline]
    pub const fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }
}

#[derive(Clone, Copy)]
pub struct SnPhysAddr(u64);

impl SnPhysAddr {
    pub const fn as_u64(&self) -> u64 {
        return self.0;
    }

    pub const fn new(addr: u64) -> Self {
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
    use ::alloc::{boxed::Box,  vec, rc::Rc, vec::Vec};
    use crate::printk;

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