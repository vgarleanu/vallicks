/// Our Tcp socket interface.
pub mod socks;
/// Our tcp stack implementation
pub mod tcp;
/// Our packet structures and parsers
pub mod wire;
/// Ethernet layer handler
pub mod ethernet;
/// Arp Layer
pub mod arp;
/// Ip layer stuff
pub mod ip;
/// Icmp layer stuff
pub mod icmp;

pub use crate::net::wire as frames;

use crate::net::tcp::*;
use crate::prelude::*;

use crate::net::socks::TcpStream;
use crate::net::wire::eth2::Ether2Frame;
use crate::net::wire::ipaddr::Ipv4Addr;
use crate::net::wire::Packet;
use crate::net::wire::mac::Mac;

use crate::net::ethernet::Ethernet;
use crate::net::arp::Arp;
use crate::net::ip::IpLayer;
use crate::net::icmp::IcmpLayer;
use crate::net::tcp::TcpLayer;

use crate::driver::NetworkDriver;
use crate::sync::mpsc::*;

use alloc::sync::Arc;
use spin::RwLock;

use hashbrown::HashMap;

use async_trait::async_trait;
use futures_util::future;
use futures_util::future::FutureExt;
use futures_util::sink::SinkExt;
use futures_util::stream::Fuse;
use futures_util::stream::StreamExt;
use lazy_static::lazy_static;

type StreamKey = TcpStream;
type OpenPorts = Arc<RwLock<HashMap<u16, UnboundedSender<StreamKey>>>>;

lazy_static! {
    pub static ref ETHERNET_LAYER: Ethernet = Ethernet::new();
    pub static ref ARP_LAYER: Arp = Arp::new();
    pub static ref IP_LAYER: IpLayer = IpLayer::new();
    pub static ref ICMP_LAYER: IcmpLayer = IcmpLayer::new();
    pub static ref TCP_LAYER: TcpLayer = TcpLayer::new();

    pub static ref OPEN_PORTS: OpenPorts = Arc::new(RwLock::new(HashMap::new()));
}

/// Trait used for parsing a packet of type `Item`. The aim of this is that in the end our network
/// stack visually looks like a state machine as well, with the idea that packets go down the
/// callstack in an obvious fashion.
#[async_trait]
trait ProcessPacket<Item> {
    /// Output packet.
    type Output: Packet;
    /// Represents a context that gets passed down to packet handler. This context is essentially the
    /// ethernet 2 frame, but in some cases could be the ipv4 packet.
    type Context: Packet;

    /// Process packet of type `Item`. This method can return an Option depending on whether we
    /// want to send a packet as a reply or not.
    async fn handle_packet(&mut self, item: Item, ctx: &Self::Context) -> Option<Self::Output>;
}

pub struct NetworkDevice<T: NetworkDriver> {
    /// Tx sink to which we can dispatch packets.
    tx_sink: T::TxSink,
    /// Rx sink from which we can receive packets.
    rx_sink: Fuse<T::RxSink>,
    /// Our ip address,
    ip: Ipv4Addr,
    /// Tx queue reader
    tx_queue: Option<UnboundedReceiver<Ether2Frame>>,
    /// Tx queue sender
    tx_queue_sender: UnboundedSender<Ether2Frame>,
    /// Device mac
    device_mac: Mac,
}

impl<T: NetworkDriver> NetworkDevice<T> {
    pub async fn new(device: &mut T) -> Self {
        // we acquire what are essentially two channels from the network driver.
        // rx_sink is for receiving ethernet ii frames from the NIC.
        // tx_sink is for sending them.
        let (rx_sink, tx_sink) = device.parts();
        // these two channels are required so that we can receive packets that need to be sent to
        // the network.
        let (tx_queue_sender, tx_queue) = channel();
        let device_mac = device.mac();

        // Register this new network device.
        ETHERNET_LAYER.register_tx(device_mac, tx_queue_sender.clone()).await;

        Self {
            rx_sink: rx_sink.fuse(),
            tx_sink,
            ip: Ipv4Addr::new(127, 0, 0, 1),
            tx_queue: Some(tx_queue),
            tx_queue_sender,
            device_mac,
        }
    }

    pub async fn set_ip(&mut self, ip: Ipv4Addr) {
        // Register our ip in the local arp table
        ARP_LAYER.register_local(ip, self.device_mac).await;
        self.ip = ip;
    }

    pub fn get_sender(&self) -> UnboundedSender<Ether2Frame> {
        self.tx_queue_sender.clone()
    }

    /// Function will run forever grabbing packets from an rx sink and processing them.
    pub async fn run_forever(&mut self) {
        let mut tx_queue = self.tx_queue.take().expect("missing tx_queue");
        loop {
            // future that will resolve to a new ether2 frame from the NIC.
            let rx_item = self.rx_sink.next();
            // future that will resolve to a new ether2 frame that we need to send to the NIC.
            let tx_item = tx_queue.recv().boxed().fuse();

            match future::select(rx_item, tx_item).await {
                future::Either::Left((item, _)) => {
                    if let Some(frame) = item {
                        if let Some(frame) = Ether2Frame::from_bytes(frame).ok() {
                            if let Some(packet) = ETHERNET_LAYER.handle_rx(frame).await {
                                let _ = self.tx_sink.send(packet.into_bytes()).await;
                                let _ = self.tx_sink.flush().await;
                            }
                        }
                    }
                }
                future::Either::Right((item, _)) => {
                    if let Some(frame) = item {
                        if let Err(tx_send_err) = self.tx_sink.send(frame.into_bytes()).await {
                            println!("net: tx_send_err {:?}", tx_send_err);
                        }

                        if let Err(tx_flush_err) = self.tx_sink.flush().await {
                            println!("net: tx_flush_err {:?}", tx_flush_err);
                        }
                    }
                }
            }
        }
    }
}
