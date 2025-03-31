#![no_std]
#![no_main]

mod panic;
mod limine;
mod fb;

pub fn kernel_main() {
    fb::init();
}
