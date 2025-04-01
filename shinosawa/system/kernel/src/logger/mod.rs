use core::fmt;

use conquer_once::spin::OnceCell;
use spin::Mutex;

use crate::{
    fb::{display::SnFramebufferDisplay, writer::SnFramebufferWriter},
    serial::SnSerialWriter,
};

/// The global logger instance used for the `log` crate.
pub static LOGGER: OnceCell<SnLogger> = OnceCell::uninit();

/// A logger instance protected by a spinlock.
pub struct SnLogger {
    pub writer: Option<Mutex<SnFramebufferWriter>>,
    pub serial: Option<Mutex<SnSerialWriter>>,
}

impl SnLogger {
    pub fn new(writer: SnFramebufferWriter, serial: SnSerialWriter) -> SnLogger {
        SnLogger {
            writer: Some(Mutex::new(writer)),
            serial: Some(Mutex::new(serial)),
        }
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
    let logger_serial = LOGGER.get().unwrap().serial.as_ref();

    let mut writer = logger_writer.unwrap().lock();
    writer.write_fmt(args).unwrap();

    let mut serial = logger_serial.unwrap().lock();
    serial.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! printk {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::println!("shinosawa::system::kernel: {}", format_args!($($arg)*)));
}

pub fn init(display: SnFramebufferDisplay, serial: SnSerialWriter) {
    let mut writer = SnFramebufferWriter::new(display);
    writer.clear();

    LOGGER.init_once(move || SnLogger::new(writer, serial));
}
