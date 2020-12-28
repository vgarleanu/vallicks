use crate::{arch::interrupts::register_interrupt, prelude::*};
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::stream::Stream;
use futures_util::task::AtomicWaker;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub struct Keyboard {
    _private: (),
}

impl Keyboard {
    pub fn new() -> Self {
        println!("keyboard: init");
        SCANCODE_QUEUE.init_once(|| ArrayQueue::new(0xff));
        register_interrupt(33, Self::handle_int);

        Self { _private: () }
    }

    pub fn init(&self) {}

    extern "x86-interrupt" fn handle_int(_: &mut InterruptStackFrame) {
        let mut port = Port::new(0x60);

        if let Ok(queue) = SCANCODE_QUEUE.try_get() {
            if let Ok(_) = queue.push(unsafe { port.read() }) {
                WAKER.wake();
            } else {
                println!("keyboard: queue full");
            }
        } else {
            println!("keyboard: queue not init");
        }

        crate::arch::interrupts::notify_eoi(33);
    }
}

impl Stream for Keyboard {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Some(x) = SCANCODE_QUEUE.try_get().unwrap().pop() {
            return Poll::Ready(Some(x));
        }

        WAKER.register(&cx.waker());

        match SCANCODE_QUEUE.try_get().unwrap().pop() {
            Some(x) => {
                WAKER.take();
                Poll::Ready(Some(x))
            }
            None => Poll::Pending,
        }
    }
}
