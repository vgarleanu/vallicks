use super::wire::ipv4::Ipv4;
use super::wire::ipv4::Ipv4Proto;
use super::wire::ipaddr::Ipv4Addr;
use super::wire::eth2::Ether2Frame;
use super::wire::icmp::Icmp;
use super::wire::tcp::Tcp;
use super::wire::Packet;
use super::wire::eth2::EtherType;

use core::sync::atomic::AtomicU16;
use core::sync::atomic::Ordering::Relaxed;

pub struct IpLayer {
    last_ipv4_id: AtomicU16,
}

impl IpLayer {
    pub fn new() -> Self {
        Self {
            last_ipv4_id: AtomicU16::new(0)
        }
    }

    pub async fn handle_packet(&self, packet: Ipv4, _: &Ether2Frame) -> Option<Ipv4> {
        // packet is malformed or not intended for us.
        if super::ARP_LAYER.resolve_ip_local(packet.dip()).await.is_none() {
            return None;
        }


        let (data, packet_type) = match packet.proto() {
            Ipv4Proto::ICMP => {
                let pkt = Icmp::from_bytes(packet.data().to_vec()).ok()?;
                (
                    super::ICMP_LAYER.handle_packet(pkt, &packet).await?.into_bytes(),
                    Ipv4Proto::ICMP,
                )
            }
            Ipv4Proto::TCP => {
                let pkt = Tcp::from_bytes(packet.data().to_vec()).ok()?;
                (
                    super::TCP_LAYER.handle_packet(pkt, &packet).await?.into_bytes(),
                    Ipv4Proto::TCP,
                )
            }
            _ => return None,
        };

        let mut reply = Ipv4::zeroed();
        reply.set_proto(packet_type);
        reply.set_sip(packet.dip());
        reply.set_dip(packet.sip());
        reply.set_id(packet.id());
        reply.set_flags(0x40);
        reply.set_data(data);
        reply.set_checksum();

        Some(reply)
    }

    pub async fn handle_tx(&self, packet: &[u8], proto: Ipv4Proto, dip: Ipv4Addr, sip: Ipv4Addr) {
        let mut ipv4 = Ipv4::zeroed();
        ipv4.set_proto(proto);
        ipv4.set_dip(dip);
        ipv4.set_sip(sip);
        ipv4.set_id(self.last_ipv4_id.fetch_add(1, Relaxed));
        ipv4.set_flags(0x40);
        ipv4.set_data(packet);
        ipv4.set_checksum();

        let dst_mac = match super::ARP_LAYER.resolve_ip(dip).await {
            Some(x) => x,
            None => return,
        };

        let src_mac = match super::ARP_LAYER.resolve_ip_local(sip).await {
            Some(x) => x,
            None => return,
        };

        let mut ether = Ether2Frame::zeroed();
        ether.set_dst(dst_mac);
        ether.set_src(src_mac);
        ether.set_dtype(EtherType::IPv4);
        ether.set_data(ipv4.into_bytes());

        super::ETHERNET_LAYER.handle_tx(ether).await;
    }
}
