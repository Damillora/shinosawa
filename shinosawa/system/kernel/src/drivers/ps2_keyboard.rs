use conquer_once::spin::OnceCell;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

use crate::{interrupt::INTERRUPT_CONTROLLER, print, printk};

const KEYBOARD_IRQ: u8 = 0x01;

static KEYBOARD: OnceCell< Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>>> = OnceCell::uninit();

fn keyboard_handler() {
    use x86_64::instructions::port::Port;

    let mut keyboard = KEYBOARD.get().unwrap().lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("#({:?})", key),
            }
        }
    }
}

pub fn init() {
    KEYBOARD.init_once(move || {
        Mutex::new(Keyboard::new(ScancodeSet1::new(),
            layouts::Us104Key, HandleControl::Ignore)
        )
    });

    // Reset anything that might have been there
    keyboard_handler();

    let mut controller = INTERRUPT_CONTROLLER.get().unwrap().write();
    controller.set_handler(KEYBOARD_IRQ as usize, keyboard_handler);

    crate::hal::interface::interrupt::enable_irq(1);
}