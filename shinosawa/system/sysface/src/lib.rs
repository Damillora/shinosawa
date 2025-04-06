#![no_std]

pub mod syscall;

pub mod linked_list;

pub mod memory;

use core::arch::asm;
use core::fmt;
use core::format_args;
use core::panic::PanicInfo;

use linked_list::LinkedListAllocator;
struct Writer {}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        syscall::write(s);
        Ok(())
    }
}

unsafe extern "C" {
    fn main() -> ();
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    Writer {}.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        _print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    println!("Oh no!\n{:?}", _info);
    loop {}
}


/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }

    pub fn try_lock(&self) -> Option<spin::MutexGuard<A>>{
        self.inner.try_lock()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn _start() -> ! {
    let heap_start: usize;
    let heap_end: usize;
    unsafe {
        asm!("",
            lateout("rax") heap_start,
            lateout("rcx") heap_end,
            options(pure, nomem, nostack)
        );
    }
    println!("heap start: {:#016X}", heap_start);
    println!("heap end: {:#016X}", heap_end);

    memory::init(heap_start, heap_end);
    unsafe { main() };

    syscall::exit();
}
