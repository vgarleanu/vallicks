use crate::sync::mpsc::*;
use crate::prelude::*;
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
        let conn = self.rx.recv().await;
        println!("received conn");
        conn
    }
}

pub struct TcpStream {
    // we send data over this
    pub(crate) tx_channel: UnboundedSender<Vec<u8>>,
    // we receive data over this
    pub(crate) rx_channel: UnboundedReceiver<Vec<u8>>,
}

impl TcpStream {
    pub async fn read(&mut self) -> Option<Vec<u8>> {
        self.rx_channel.recv().await
    }

    pub fn write(&mut self, item: Vec<u8>) {
        self.tx_channel.send(item).unwrap();
    }
}
