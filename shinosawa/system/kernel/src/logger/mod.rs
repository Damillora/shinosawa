use core::fmt::{self, Write};

use conquer_once::spin::OnceCell;
use logbuf::SnLogBuffer;
use spin::{Mutex, RwLock};

use crate::{
    fb::{display::SnFramebufferDisplay, writer::SnFramebufferWriter}, hal, serial::SnSerialWriter
};

pub mod logbuf;

/// The global logger instance used for the `log` crate.
pub static LOGGER: OnceCell<RwLock<SnLogger>> = OnceCell::uninit();

/// A logger instance protected by a spinlock.
pub struct SnLogger {
    pub fb: Option<Mutex<SnFramebufferWriter>>,
    pub serial: Option<Mutex<SnSerialWriter>>,
    pub buf: Option<Mutex<SnLogBuffer>>,
}

impl SnLogger {
    pub fn new() -> SnLogger {
        SnLogger {
            fb: None,
            serial: None,
            buf: None,
        }
    }

    pub fn add_buffer(&mut self, writer: SnLogBuffer) {
        self.buf = Some(Mutex::new(writer));
    }

    pub fn add_fb(&mut self, writer: SnFramebufferWriter) {
        self.fb = Some(Mutex::new(writer));
    }

    pub fn add_serial(&mut self, serial: SnSerialWriter) {
        self.serial = Some(Mutex::new(serial));
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
    if !LOGGER.is_initialized() { return }
    
    use core::fmt::Write;
    let logger = LOGGER.get().unwrap().read();
    hal::interface::interrupt::without_interrupts(|| {
        if let Some(log_buffer) = &logger.buf {
            let mut buffer = log_buffer.lock();
            buffer.write_fmt(args).unwrap();
        }

        if let Some(logger_writer) = &logger.fb {
            let mut writer = logger_writer.lock();
            writer.write_fmt(args).unwrap();
        }
        if let Some(logger_serial) = &logger.serial {
            let mut serial = logger_serial.lock();
            serial.write_fmt(args).unwrap();
        }
    });
}

#[macro_export]
macro_rules! print_s {
    ($($arg:tt)*) => ($crate::logger::_print_serial(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! printk {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::println!("shinosawa::system::kernel: {}", format_args!($($arg)*)));
}

pub fn init() {
    LOGGER.init_once(move || RwLock::new(SnLogger::new()));
}

pub fn set_buffer(buffer: SnLogBuffer) {
    let mut logger = LOGGER.get().unwrap().write();

    logger.add_buffer(buffer);
}

pub fn set_fb(display: SnFramebufferDisplay) {
    let mut writer = SnFramebufferWriter::new(display);
    writer.clear();

    let mut logger = LOGGER.get().unwrap().write();

    logger.add_fb(writer);
}

pub fn set_serial(serial: SnSerialWriter) {
    let mut logger = LOGGER.get().unwrap().write();

    logger.add_serial(serial);
}

pub fn clean_buffer() {
    let logger = LOGGER.get().unwrap().write();

    if let Some(buf) = &logger.buf {
        let mut buffer = buf.lock();

        buffer.iter().for_each(|c| {
            hal::interface::interrupt::without_interrupts(|| {
                if let Some(logger_writer) = &logger.fb {
                    let mut writer = logger_writer.lock();
                    writer.write_char(*c).unwrap();
                }
                if let Some(logger_serial) = &logger.serial {
                    let mut serial = logger_serial.lock();
                    serial.write_char(*c).unwrap();
                }
            });
        });
    }
}
