use super::wire::arp::ArpPacket;
use super::wire::arp::ArpOpcode;
use super::wire::eth2::Ether2Frame;
use super::wire::mac::Mac;
use super::wire::ipaddr::Ipv4Addr;
use super::wire::Packet;
use super::wire::eth2::EtherType;

use core::time::Duration;
use crate::async_::Sleep;
use crate::collections::HashMap;
use crate::sync::RwLock;

/// Struct represents the Arp layer of our network stack. As such all arp packets are proccessed by
/// a static instance of this struct.
pub struct Arp {
    /// Hashmap maps ip addresses mapped to macs.
    arp_table: RwLock<HashMap<Mac, Ipv4Addr>>,
    /// Hashmap of local ips mapped to device macs.
    local_arp_table: RwLock<HashMap<Ipv4Addr, Mac>>,
}

impl Arp {
    pub fn new() -> Self {
        Self {
            arp_table: RwLock::new(HashMap::new()),
            local_arp_table: RwLock::new(HashMap::new()),
        }
    }

    pub async fn handle_packet(&self, packet: ArpPacket, _: &Ether2Frame) -> Option<ArpPacket> {
        let local_mac = self.local_arp_table.read().await.get(&packet.tip())?.clone();

        self.arp_table.write().await.insert(packet.smac(), packet.sip());

        let mut reply = packet.clone();
        reply.set_tmac(reply.smac());
        reply.set_smac(local_mac);
        reply.set_tip(reply.sip());
        reply.set_sip(packet.tip());
        reply.set_opcode(ArpOpcode::ArpReply);

        Some(reply)
    }

    pub async fn register_local(&self, lip: Ipv4Addr, lmac: Mac) {
        self.local_arp_table.write().await.insert(lip, lmac);
    }

    pub async fn resolve_ip(&self, ip: Ipv4Addr, local: Ipv4Addr) -> Option<Mac> {
        // First check our local tables for whether we already have an entry.
        self.arp_table.read().await.iter().find(|(_, x)| **x == ip).map(|(mac, _)| *mac);
        // Get our local mac
        let local_mac = self.local_arp_table.read().await.get(&local)?.clone();
        
        for _ in 0usize..5 {
            self.arp_query(ip, local, local_mac).await;

            Sleep::new(Duration::from_millis(1000)).await;

            if let Some(x) = self.arp_table.read().await.iter().find(|(_, x)| **x == ip).map(|(mac, _)| *mac) {
                return Some(x);
            }
        }

        // TODO: If we get here that means we have timeouted and we must notify the client maybe??
        None
    }

    pub async fn resolve_ip_local(&self, ip: Ipv4Addr) -> Option<Mac> {
        self.local_arp_table.read().await.iter().find(|(x, _)| **x == ip).map(|(_, mac)| *mac)
    }

    pub async fn arp_query(&self, ip: Ipv4Addr, local_ip: Ipv4Addr, local_mac: Mac) {
        let mut request = ArpPacket::zeroed();
        request.set_tmac(Mac::multicast());
        request.set_smac(local_mac);
        request.set_tip(ip);
        request.set_sip(local_ip);
        request.set_opcode(ArpOpcode::ArpRequest);

        let mut ethpacket = Ether2Frame::zeroed();
        ethpacket.set_dst(Mac::multicast());
        ethpacket.set_src(local_mac);
        ethpacket.set_dtype(EtherType::ARP);
        ethpacket.set_data(request.as_ref());

        super::ETHERNET_LAYER.handle_tx(ethpacket).await;
    }
}
