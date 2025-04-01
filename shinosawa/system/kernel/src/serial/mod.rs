use core::fmt;

pub struct SnSerialWriter {
    port: uart_16550::SerialPort,
}

impl SnSerialWriter {
    /// # Safety
    ///
    /// unsafe because this function must only be called once
    unsafe fn init() -> Self {
        let mut port = unsafe { uart_16550::SerialPort::new(0x3F8) };
        port.init();
        Self { port }
    }
}

pub unsafe fn init() -> SnSerialWriter {
    unsafe { SnSerialWriter::init() }
}

impl fmt::Write for SnSerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for char in s.bytes() {
            match char {
                b'\n' => self.port.write_str("\r\n").unwrap(),
                byte => self.port.send(byte),
            }
        }
        Ok(())
    }
}
