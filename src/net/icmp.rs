use super::wire::icmp::Icmp;
use super::wire::icmp::IcmpType;
use super::wire::ipv4::Ipv4;

pub struct IcmpLayer;

impl IcmpLayer {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle_packet(&self, packet: Icmp, _: &Ipv4) -> Option<Icmp> {
        match packet.packet_type() {
            IcmpType::Echo => {
                let mut reply = packet.clone();
                reply.set_packet_type(IcmpType::EchoReply);
                reply.set_checksum();
                Some(reply)
            }
            _ => None,
        }
    }
}
