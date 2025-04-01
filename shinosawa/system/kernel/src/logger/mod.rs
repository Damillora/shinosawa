use core::fmt;

use conquer_once::spin::OnceCell;
use spinning_top::Spinlock;

use crate::{
    fb::writer::SnFramebufferWriter,
    serial::SnSerialWriter,
};

/// The global logger instance used for the `log` crate.
pub static LOGGER: OnceCell<SnLogger> = OnceCell::uninit();

/// A logger instance protected by a spinlock.
pub struct SnLogger {
    pub writer: Option<Spinlock<SnFramebufferWriter>>,
    pub serial: Option<Spinlock<SnSerialWriter>>,
}

impl SnLogger {
    pub fn new(writer: SnFramebufferWriter, serial: SnSerialWriter) -> SnLogger {
        SnLogger { writer: Some(Spinlock::new(writer)), serial: Some(Spinlock::new(serial)) }
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

    if let Some(logger_writer) = LOGGER.get() {
        if let Some(writer) = logger_writer.writer.as_ref() {
            let mut writer = writer.lock();
            writer.write_fmt(args).unwrap();
        }

        if let Some(serial) = logger_writer.serial.as_ref() {
            let mut serial = serial.lock();
            serial.write_fmt(args).unwrap();
        }
    }
}

#[macro_export]
macro_rules! printk {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::println!("shinosawa::system::kernel: {}", format_args!($($arg)*)));
}

pub fn init(writer: SnFramebufferWriter, serial_writer: SnSerialWriter) {
    let log = SnLogger::new(writer,serial_writer);

    LOGGER.init_once(move || log );
}
