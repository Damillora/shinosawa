use crate::printk;

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    printk!("PANIC: {}\n{:#?}", info.message().as_str().unwrap_or("See info below"), info);
    use crate::hal::interface::instruct::hcf;
    hcf();
}
