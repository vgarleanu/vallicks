use crate::sync::mpsc::*;
use core::time::Duration;
use crate::prelude::*;
use crate::sync::Arc;
use crate::sync::RwLock;
use super::OPEN_PORTS;
use super::StreamKey;

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

        Ok(Self {
            rx
        })
    }

    pub async fn accept(&mut self) -> Option<TcpStream> {
        self.rx.recv().await
    }
}

pub struct TcpStream {
    pub(crate) raw_connection: Arc<RwLock<super::TcpConnection>>,
}

impl TcpStream {
    pub async fn read(&mut self, buffer: &mut [u8]) -> usize{
        self.raw_connection.write().await.read(buffer)
    }

    pub fn write(&mut self, item: Vec<u8>) {
        unimplemented!()
    }
}
