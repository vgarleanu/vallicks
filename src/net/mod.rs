/// Our packet structures and parsers
pub mod wire;

pub use crate::net::wire as frames;

use crate::prelude::*;

use crate::net::wire::arp::{ArpOpcode, ArpPacket};
use crate::net::wire::eth2::{Ether2Frame, EtherType};
use crate::net::wire::icmp::{Icmp, IcmpType};
use crate::net::wire::ipaddr::Ipv4Addr;
use crate::net::wire::ipv4::{Ipv4, Ipv4Proto};
use crate::net::wire::mac::Mac;

use crate::driver::NetworkDriver;

use hashbrown::HashMap;

use futures_util::sink::SinkExt;
use futures_util::stream::Fuse;
use futures_util::stream::StreamExt;
use futures_util::select_biased;

/// Trait used for parsing a packet of type `Item`.
trait ProcessPacket<Item> {
    /// Output packet.
    type Output;

    /// Process packet of type `Item`. This method can return an Option depending on whether we
    /// want to send a packet as a reply or not.
    fn process_packet(&mut self, item: Item) -> Option<Self::Output>;
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
}

impl<T: NetworkDriver> NetworkDevice<T> {
    pub fn new(device: &mut T) -> Self {
        let (rx_sink, tx_sink) = device.parts();
        Self {
            rx_sink: rx_sink.fuse(),
            tx_sink,
            mac: device.mac(),
            arp_translation_table: HashMap::new(),
            ip: Ipv4Addr::new(127, 0, 0, 1),
        }
    }

    pub fn set_ip(&mut self, ip: Ipv4Addr) {
        self.ip = ip;
    }

    /// Function will run forever grabbing packets from an rx sink and processing them.
    pub async fn run_forever(&mut self) {
        loop {
            let mut rx_item = self.rx_sink.next();
            let mut tx_item = futures_util::future::pending::<()>();

            select_biased! {
                // We have an incoming packet which needs to be handle.
                frame = rx_item => {
                    if let Some(frame) = frame {
                        self.try_handle_tx(frame).await;
                    }
                },

                item = tx_item => {
                    println!("got tx");
                }
            }
        }
    }

    async fn try_handle_tx(&mut self, frame: Ether2Frame) {
        match frame.dtype() {
            EtherType::IPv4 => {
                let ip_packet = Ipv4::from(frame.data().to_vec()).unwrap();
                if let Some(x) = self.process_packet(ip_packet) {
                    let mut reply = Ether2Frame::new();
                    reply.set_dst(frame.src());
                    reply.set_src(self.mac);
                    reply.set_dtype(EtherType::IPv4);
                    reply.set_data(x.into_inner());

                    let _ = self.tx_sink.send(reply.into_inner()).await;
                    let _ = self.tx_sink.flush().await;
                }
            }
            EtherType::ARP => {
                let arp_packet = ArpPacket::from(frame.data().to_vec()).unwrap();
                if let Some(x) = self.process_packet(arp_packet) {
                    let _ = self.tx_sink.send(x.into_inner()).await;
                    let _ = self.tx_sink.flush().await;
                }
            }
            EtherType::Unsupported => {}
        }
    }
}

impl<T: NetworkDriver> ProcessPacket<ArpPacket> for NetworkDevice<T> {
    type Output = Ether2Frame;

    fn process_packet(&mut self, item: ArpPacket) -> Option<Self::Output> {
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

        let mut reply_frame = Ether2Frame::new();
        reply_frame.set_dst(item.smac());
        reply_frame.set_src(self.mac);
        reply_frame.set_dtype(EtherType::ARP);
        reply_frame.set_data(reply);

        Some(reply_frame)
    }
}

impl<T: NetworkDriver> ProcessPacket<Ipv4> for NetworkDevice<T> {
    type Output = Ipv4;

    fn process_packet(&mut self, item: Ipv4) -> Option<Self::Output> {
        // packet is malformed or not intended for us.
        if item.dip() != self.ip {
            return None;
        }

        match item.proto() {
            Ipv4Proto::ICMP => {
                let packet = Icmp::from(item.data().to_vec()).ok()?;

                return self.process_packet(packet).map(|data| {
                    let mut packet = Ipv4::new_v4();
                    packet.set_proto(Ipv4Proto::ICMP);
                    packet.set_sip(self.ip);
                    packet.set_dip(item.sip());
                    packet.set_id(item.id());
                    packet.set_flags(0x40);
                    packet.set_data(data.into_inner());
                    packet.set_len();
                    packet.set_checksum();
                    packet
                });
            }
            _ => {
                println!("attempted to handle unimp packet");
            }
        }
        None
    }
}

impl<T: NetworkDriver> ProcessPacket<Icmp> for NetworkDevice<T> {
    type Output = Icmp;

    fn process_packet(&mut self, item: Icmp) -> Option<Self::Output> {
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
