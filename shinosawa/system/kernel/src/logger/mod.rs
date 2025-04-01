use core::fmt;

use conquer_once::spin::OnceCell;
use spinning_top::Spinlock;

use crate::fb::{display::SnFramebufferDisplay, writer::SnFramebufferWriter};


/// The global logger instance used for the `log` crate.
pub static LOGGER: OnceCell<SnLogger> = OnceCell::uninit();

/// A logger instance protected by a spinlock.
pub struct SnLogger {
    pub writer: Option<Spinlock<SnFramebufferWriter>>,

}

impl SnLogger {
    pub fn new(writer: SnFramebufferWriter) -> SnLogger {
        SnLogger { writer: Some(Spinlock::new(writer)) }
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::logger::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    let logger_writer = LOGGER.get().unwrap().writer.as_ref();

    let mut writer = logger_writer.unwrap().lock();
    writer.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! printk {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::println!("shinosawa::system::kernel: {}", format_args!($($arg)*)));
}

pub fn init(display: SnFramebufferDisplay) {
    let mut writer = SnFramebufferWriter::new(display);
    writer.clear();

    LOGGER.init_once(move || SnLogger::new(writer));
}