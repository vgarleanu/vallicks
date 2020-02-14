use crate::{arch::interrupts::register_interrupt, prelude::*};
use crate::prelude::sync::{RwLock, Arc};
use arraydeque::{ArrayDeque, Wrapping};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard as KeyboardBackend, ScancodeSet1};
use x86_64::instructions::port::Port;

pub struct Keyboard {
    inner: Arc<RwLock<KeyboardInner>>,
}

struct KeyboardInner {
    backend: KeyboardBackend<layouts::Us104Key, ScancodeSet1>,
    input_buffer: ArrayDeque<[char; 64], Wrapping>,
    port: Port<u8>,
}

impl KeyboardInner {
    fn new() -> Self {
        Self {
            backend: KeyboardBackend::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore),
            input_buffer: ArrayDeque::new(),
            port: Port::new(0x60),
        }
    }

    fn handle_int(&mut self) {
        if let Ok(Some(key_event)) = self.backend.add_byte(unsafe { self.port.read() }) {
            if let Some(DecodedKey::Unicode(key)) = self.backend.process_keyevent(key_event) {
                if !key.is_ascii() {
                    return;
                }

                self.input_buffer.push_back(key);
            }
        }
    }
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(KeyboardInner::new())),
        }
    }

    pub fn init(&self) {
        println!("keyboard: Registering IRQ");
        let inner = self.inner.clone();
        register_interrupt(33, move || Self::handle_int(inner.clone()));
    }

    fn handle_int(inner: Arc<RwLock<KeyboardInner>>) {
        inner.write().handle_int();
    }
}
