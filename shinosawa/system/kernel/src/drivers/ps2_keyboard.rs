use crate::{interrupt::INTERRUPT_CONTROLLER, printk};

const KEYBOARD_IRQ: u8 = 0x01;

fn keyboard_handler() {
    
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    
    printk!("x86_64: keypress: {}", scancode);
}

pub fn init() {
    let mut controller = INTERRUPT_CONTROLLER.get().unwrap().write();
    controller.set_handler(KEYBOARD_IRQ as usize, keyboard_handler);

    crate::hal::interface::interrupt::enable_irq(1);
}