use super::wire::arp::ArpPacket;
use super::wire::arp::ArpOpcode;
use super::wire::eth2::Ether2Frame;
use crate::sync::RwLock;
use super::wire::mac::Mac;
use super::wire::ipaddr::Ipv4Addr;
use crate::collections::HashMap;

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

    pub async fn handle_packet(&self, packet: ArpPacket, ctx: &Ether2Frame) -> Option<ArpPacket> {
        if packet.tmac() != ctx.dst() {
            if !self.local_arp_table.read().await.contains_key(&packet.tip()) {
                return None;
            }
        }

        if packet.opcode() == ArpOpcode::ArpReply {
            self.arp_table.write().await.insert(packet.smac(), packet.sip());
            return None;
        }

        let mut reply = packet.clone();
        reply.set_smac(packet.tmac());
        reply.set_tmac(reply.smac());
        reply.set_tip(reply.sip());
        reply.set_sip(packet.tip());
        reply.set_opcode(ArpOpcode::ArpReply);

        Some(reply)
    }

    pub async fn register_local(&self, lip: Ipv4Addr, lmac: Mac) {
        self.local_arp_table.write().await.insert(lip, lmac);
    }

    pub async fn resolve_ip(&self, ip: Ipv4Addr) -> Option<Mac> {
        // TODO: resolve ip by looking it up in our translation table, if its not there, send a
        // query to the network.
        self.arp_table.read().await.iter().find(|(_, x)| **x == ip).map(|(mac, _)| *mac)
    }

    pub async fn resolve_ip_local(&self, ip: Ipv4Addr) -> Option<Mac> {
        self.local_arp_table.read().await.iter().find(|(x, _)| **x == ip).map(|(_, mac)| *mac)
    }
}
