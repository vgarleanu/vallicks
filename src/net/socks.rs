use super::StreamKey;
use super::OPEN_PORTS;
use crate::prelude::*;
use crate::sync::mpsc::*;
use crate::sync::Arc;
use crate::sync::Mutex;

use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

pub struct TcpListener {
    rx: UnboundedReceiver<StreamKey>,
}

impl TcpListener {
    pub fn bind(port: u16) -> Result<Self, ()> {
        let (tx, rx) = channel();
        {
            let mut ports = OPEN_PORTS.write();

            if ports.contains_key(&port) {
                return Err(());
            }

            ports.insert(port, tx);
        }

        Ok(Self { rx })
    }

    pub async fn accept(&mut self) -> Option<TcpStream> {
        self.rx.recv().await
    }
}

pub struct TcpStream {
    pub(crate) raw: Arc<Mutex<super::TcpConnection>>,
}

impl TcpStream {
    pub async fn read(&mut self, buffer: &mut [u8]) -> usize {
        struct ReadFuture<'a> {
            inner: &'a TcpStream,
            buffer: &'a mut [u8],
        }

        impl<'a> Future for ReadFuture<'a> {
            type Output = usize;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.inner.raw.try_lock() {
                    Some(mut guard) => {
                        if !guard.has_data() {
                            guard.register_waker(cx.waker().clone());
                            return Poll::Pending;
                        }

                        return Poll::Ready(guard.read(self.buffer));
                    }
                    None => {
                        self.inner.raw.register_waker(cx);
                        return Poll::Pending;
                    }
                }
            }
        }

        ReadFuture {
            inner: self,
            buffer,
        }
        .await
    }

    pub async fn write(&mut self, item: &[u8]) {
        self.raw.lock().await.write(item);
    }
}
