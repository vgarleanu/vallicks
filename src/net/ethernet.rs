use crate::collections::HashMap;
use crate::sync::Arc;
use crate::sync::RwLock;
use crate::sync::mpsc::UnboundedSender;
use super::wire::eth2::Ether2Frame;
use super::wire::mac::Mac;
use super::wire::ipv4::Ipv4;
use super::wire::arp::ArpPacket;
use super::wire::eth2::EtherType;
use super::wire::Packet;

type TxQueueSender = UnboundedSender<Ether2Frame>;

pub struct Ethernet {
    tx_queue_map: RwLock<HashMap<Mac, TxQueueSender>>,
}

impl Ethernet {
    pub fn new() -> Self {
        Self {
            tx_queue_map: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register_tx(&self, device_mac: Mac, tx_queue: TxQueueSender) {
        self.tx_queue_map
            .write()
            .await
            .insert(device_mac, tx_queue);
    }

    /// Function handles an incoming packet.
    pub async fn handle_rx(&self, ctx: Ether2Frame, device_mac: Mac) -> Option<Ether2Frame> {
        let (data, frame_type) = match ctx.dtype() {
            EtherType::IPv4 => {
                let pkt = Ipv4::from_bytes(ctx.data().to_vec()).ok()?;
                (
                    super::IP_LAYER.handle_packet(pkt, &ctx).await?.into_bytes(),
                    EtherType::IPv4
                )
            },
            EtherType::ARP => {
                let pkt = ArpPacket::from_bytes(ctx.data().to_vec()).ok()?;
                (
                    super::ARP_LAYER.handle_packet(pkt, &ctx).await?.into_bytes(),
                    EtherType::ARP,
                )
            }
            _ => {
                return None;
            }
        };

        let mut reply = Ether2Frame::zeroed();
        reply.set_dst(ctx.src());
        reply.set_src(device_mac);
        reply.set_dtype(frame_type);
        reply.set_data(data);

        Some(reply)
    }

    /// Function can be used to send data out.
    pub async fn handle_tx(&self, packet: Ether2Frame) {
        if let Some(lock) = self.tx_queue_map.read().await.get(&packet.dst()) {
            lock.send(packet).expect("eth2: failed to write to netdev");
        }
    }
}
