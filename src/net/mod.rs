/// Our packet structures and parsers
pub mod wire;
/// Our tcp stack implementation
pub mod tcp;

pub use crate::net::wire as frames;

use crate::prelude::*;

use crate::net::wire::arp::{ArpOpcode, ArpPacket};
use crate::net::wire::eth2::{Ether2Frame, EtherType};
use crate::net::wire::icmp::{Icmp, IcmpType};
use crate::net::wire::ipaddr::Ipv4Addr;
use crate::net::wire::ipv4::{Ipv4, Ipv4Proto};
use crate::net::wire::tcp::TcpFlag;
use crate::net::wire::mac::Mac;
use crate::net::wire::tcp::Tcp;
use crate::net::wire::Packet;

use crate::driver::NetworkDriver;
use crate::sync::mpsc::*;

use hashbrown::hash_map::Entry;
use hashbrown::HashMap;

use futures_util::sink::SinkExt;
use futures_util::stream::Fuse;
use futures_util::stream::StreamExt;
use futures_util::future::FutureExt;
use futures_util::future;

use crate::net::tcp::*;

/// Trait used for parsing a packet of type `Item`. The aim of this is that in the end our network
/// stack visually looks like a state machine as well, with the idea that packets go down the
/// callstack in an obvious fashion.
trait ProcessPacket<Item> {
    /// Output packet.
    type Output: Packet;
    /// Represents a context that gets passed down to packet handler. This context is essentially the
    /// ethernet 2 frame, but in some cases could be the ipv4 packet.
    type Context: Packet;

    /// Process packet of type `Item`. This method can return an Option depending on whether we
    /// want to send a packet as a reply or not.
    fn handle_packet(&mut self, item: Item, ctx: &Self::Context) -> Option<Self::Output>;
}

pub struct NetworkDevice<T: NetworkDriver> {
    /// Tx sink to which we can dispatch packets.
    tx_sink: T::TxSink,
    /// Rx sink from which we can receive packets.
    rx_sink: Fuse<T::RxSink>,
    /// The mac address of the device being wrapped.
    mac: Mac,
    /// Our ip address,
    ip: Ipv4Addr,
    /// Translation table for arp
    arp_translation_table: HashMap<Mac, Ipv4Addr>,
    /// Tx queue reader
    tx_queue: Option<UnboundedReceiver<Ether2Frame>>,
    /// Tx queue sender
    tx_queue_sender: UnboundedSender<Ether2Frame>,
    
    // ** TCP STACK STARTS HERE **
    /// TCP Connection map.
    tcp_map: ConnectionMap,
}

impl<T: NetworkDriver> NetworkDevice<T> {
    pub fn new(device: &mut T) -> Self {
        // we acquire what are essentially two channels from the network driver.
        // rx_sink is for receiving ethernet ii frames from the NIC.
        // tx_sink is for sending them.
        let (rx_sink, tx_sink) = device.parts();
        // these two channels are required so that we can receive packets that need to be sent to
        // the network.
        let (tx_queue_sender, tx_queue) = channel();

        Self {
            rx_sink: rx_sink.fuse(),
            tx_sink,
            mac: device.mac(),
            arp_translation_table: HashMap::new(),
            ip: Ipv4Addr::new(127, 0, 0, 1),
            tx_queue: Some(tx_queue), tx_queue_sender,
            tcp_map: ConnectionMap::new(),
        }
    }

    pub fn set_ip(&mut self, ip: Ipv4Addr) {
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
                future::Either::Left((item, _)) => if let Some(frame) = item {
                    if let Some(packet) = self.handle_packet(frame, &()) {
                        let _ = self.tx_sink.send(packet.into_bytes()).await;
                        let _ = self.tx_sink.flush().await;
                    }
                },
                future::Either::Right((item, _)) => if let Some(frame) = item {
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

impl<T: NetworkDriver> ProcessPacket<Ether2Frame> for NetworkDevice<T> {
    type Output = Ether2Frame;
    type Context = ();

    fn handle_packet(&mut self, item: Ether2Frame, _: &Self::Context) -> Option<Self::Output> {
        let (data, frame_type) = match item.dtype() {
            EtherType::IPv4 => {
                let packet = Ipv4::from_bytes(item.data().to_vec()).ok()?;
                (self.handle_packet(packet, &item)?.into_bytes(), EtherType::IPv4)
            }
            EtherType::ARP => {
                let packet = ArpPacket::from_bytes(item.data().to_vec()).ok()?;
                (self.handle_packet(packet, &item)?.into_bytes(), EtherType::ARP)
            }
            EtherType::Unsupported => {
                return None;
            }
        };

        let mut reply = Ether2Frame::zeroed();
        reply.set_dst(item.src());
        reply.set_src(self.mac);
        reply.set_dtype(frame_type);
        reply.set_data(data);

        Some(reply)
    }
}

impl<T: NetworkDriver> ProcessPacket<ArpPacket> for NetworkDevice<T> {
    type Output = ArpPacket;
    type Context = Ether2Frame;

    fn handle_packet(&mut self, item: ArpPacket, _: &Self::Context) -> Option<Self::Output> {
        if item.tmac() != self.mac && item.tip() != self.ip {
            return None;
        }

        if item.opcode() == ArpOpcode::ArpReply {
            self.arp_translation_table.insert(item.smac(), item.sip());
            return None;
        }

        let mut reply = item.clone();
        reply.set_tmac(reply.smac());
        reply.set_smac(self.mac);
        reply.set_tip(reply.sip());
        reply.set_sip(self.ip);
        reply.set_opcode(ArpOpcode::ArpReply);

        Some(reply)
    }
}

impl<T: NetworkDriver> ProcessPacket<Ipv4> for NetworkDevice<T> {
    type Output = Ipv4;
    type Context = Ether2Frame;

    fn handle_packet(&mut self, item: Ipv4, _: &Self::Context) -> Option<Self::Output> {
        // packet is malformed or not intended for us.
        if item.dip() != self.ip {
            return None;
        }

        let (data, packet_type) = match item.proto() {
            Ipv4Proto::ICMP => {
                let packet = Icmp::from_bytes(item.data().to_vec()).ok()?;
                (self.handle_packet(packet, &item)?.into_bytes(), Ipv4Proto::ICMP)
            }
            Ipv4Proto::TCP => {
                let packet = Tcp::from_bytes(item.data().to_vec()).ok()?;
                (self.handle_packet(packet, &item)?.into_bytes(), Ipv4Proto::TCP)
            }
            _ => {
                return None
            }
        };

        let mut reply = Ipv4::zeroed();
        reply.set_proto(packet_type);
        reply.set_sip(self.ip);
        reply.set_dip(item.sip());
        reply.set_id(item.id());
        reply.set_flags(0x40);
        reply.set_data(data);
        reply.set_len();
        reply.set_checksum();

        Some(reply)
    }
}

impl<T: NetworkDriver> ProcessPacket<Icmp> for NetworkDevice<T> {
    type Output = Icmp;
    type Context = Ipv4;

    fn handle_packet(&mut self, item: Icmp, _: &Self::Context) -> Option<Self::Output> {
        match item.packet_type() {
            IcmpType::Echo => {
                let mut reply = item.clone();
                reply.set_packet_type(IcmpType::EchoReply);
                reply.set_checksum();
                Some(reply)
            }
            _ => None,
        }
    }
}

impl<T: NetworkDriver> ProcessPacket<Tcp> for NetworkDevice<T> {
    type Output = Tcp;
    type Context = Ipv4;

    fn handle_packet(&mut self, item: Tcp, ctx: &Self::Context) -> Option<Self::Output> {
        let conn_key = (ctx.sip(), item.src_port(), ctx.dip(), item.dst_port());

        match self.tcp_map.entry(conn_key) {
            Entry::Occupied(mut entry) => {
                return entry.get_mut().handle_packet(item, ctx);
            },
            Entry::Vacant(entry) => {
                let (connection, tx) = TcpConnection::accept(item, ctx)?;
                entry.insert(connection);
                return Some(tx);
            },
        }
    }
}
