/// Our packet structures and parsers
pub mod wire;

pub use crate::net::wire as frames;

use crate::prelude::*;

use crate::net::wire::arp::{ArpOpcode, ArpPacket};
use crate::net::wire::eth2::{Ether2Frame, EtherType};
use crate::net::wire::icmp::{Icmp, IcmpCode, IcmpType};
use crate::net::wire::ipaddr::Ipv4Addr;
use crate::net::wire::ipv4::{Ipv4, Ipv4Proto};
use crate::net::wire::mac::Mac;

use alloc::vec::Vec;
use hashbrown::HashMap;

use futures_util::sink::Sink;
use futures_util::sink::SinkExt;
use futures_util::stream::Stream;
use futures_util::stream::StreamExt;

use core::convert::TryInto;

/// Trait used to mark a network device driver.
pub trait StreamSplit {
    /// Stream from where we can acquire new ether2 frames.
    type RxSink: Stream<Item = Ether2Frame> + Unpin;
    /// Sink where we can dispatch packets.
    type TxSink: Sink<Vec<u8>, Error = ()> + Unpin;

    /// Split current device driver into a rx sink and a tx sink.
    fn split(&mut self) -> (Self::RxSink, Self::TxSink);
    /// Mac address of the device.
    fn mac(&self) -> Mac;
}

/// Trait used for parsing a packet of type `Item`.
trait ProcessPacket<Item> {
    /// Output packet.
    type Output;

    /// Process packet of type `Item`. This method can return an Option depending on whether we
    /// want to send a packet as a reply or not.
    fn process_packet(&mut self, item: Item) -> Option<Self::Output>;
}

pub struct NetworkDevice<T: StreamSplit> {
    /// Tx sink to which we can dispatch packets.
    tx_sink: <T as StreamSplit>::TxSink,
    /// Rx sink from which we can receive packets.
    rx_sink: <T as StreamSplit>::RxSink,
    /// The mac address of the device being wrapped.
    mac: Mac,
    /// Our ip address,
    ip: Ipv4Addr,
    /// Translation table for arp
    arp_translation_table: HashMap<Mac, Ipv4Addr>,
}

impl<T: StreamSplit> NetworkDevice<T> {
    pub fn new(device: &mut T) -> Self {
        let (rx_sink, tx_sink) = device.split();
        Self {
            rx_sink,
            tx_sink,
            mac: device.mac(),
            arp_translation_table: HashMap::new(),
            ip: Ipv4Addr::new(127, 0, 0, 1),
        }
    }

    pub fn set_ip(&mut self, ip: Ipv4Addr) {
        self.ip = ip;
    }

    pub async fn process(&mut self) {
        while let Some(frame) = self.rx_sink.next().await {
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
}

impl<T: StreamSplit> ProcessPacket<ArpPacket> for NetworkDevice<T> {
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

impl<T: StreamSplit> ProcessPacket<Ipv4> for NetworkDevice<T> {
    type Output = Ipv4;

    fn process_packet(&mut self, item: Ipv4) -> Option<Self::Output> {
        // packet is malformed or not intended for us.
        if item.dip() != self.ip {
            return None;
        }

        match item.proto() {
            Ipv4Proto::ICMP => {
                let packet: Icmp = item.data().try_into().unwrap();

                return self.process_packet(packet).map(|data| {
                    let data: Vec<u8> = data.into();
                    let mut packet = Ipv4::new_v4();
                    packet.set_proto(Ipv4Proto::ICMP);
                    packet.set_sip(self.ip);
                    packet.set_dip(item.sip());
                    packet.set_id(item.id());
                    packet.set_data(data);
                    packet.set_len();
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

impl<T: StreamSplit> ProcessPacket<Icmp> for NetworkDevice<T> {
    type Output = Icmp;

    fn process_packet(&mut self, item: Icmp) -> Option<Self::Output> {
        match item {
            Icmp::Echo {
                packet_type,
                code,
                checksum,
                identifier,
                sequence_number,
                data,
            } => match packet_type {
                IcmpType::Echo => Some(Icmp::Echo {
                    packet_type: IcmpType::EchoReply,
                    code,
                    checksum,
                    identifier,
                    sequence_number,
                    data,
                }),
                _ => None,
            },
        }
    }
}
