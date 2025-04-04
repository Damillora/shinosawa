use core::fmt;

use ringbuffer::{AllocRingBuffer, RingBuffer};

const BUF_SIZE: usize = 16 * 1024; // 16 KiB

pub struct SnLogBuffer {
    buf: AllocRingBuffer<char>,
}

impl SnLogBuffer {
    pub fn new() -> SnLogBuffer {
        SnLogBuffer {
            buf: AllocRingBuffer::new(BUF_SIZE),
        }
    }

    pub fn drain(&mut self) -> impl Iterator<Item = char> {
        self.buf.drain()
    }
}

impl fmt::Write for SnLogBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.buf.push(c);
        }
        
        Ok(())
    }
}
