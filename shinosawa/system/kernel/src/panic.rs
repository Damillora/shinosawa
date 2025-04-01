use crate::printk;

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    printk!("PANIC: {}", _info.message().as_str().unwrap());

    use crate::hal::interface::instruct::hcf;
    hcf();
}
