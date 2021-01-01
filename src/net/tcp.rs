use super::wire::ipaddr::Ipv4Addr;
use super::wire::ipv4::Ipv4;
use super::wire::tcp::Tcp;
use super::wire::tcp::TcpFlag;
use super::wire::tcp::TcpStates;

use crate::prelude::*;
use hashbrown::HashMap;

pub type ConnectionKey = (Ipv4Addr, u16, Ipv4Addr, u16); // sip, sport, dip, dport
pub type ConnectionMap = HashMap<ConnectionKey, TcpConnection>;

pub struct TcpConnection {
    /// Current state of this tcp connection
    state: TcpStates,
    /// Magic numbers needed to send packets
    tx_seqspace: TxSequenceSpace,
    /// Magic numbers needed to receive packets.
    rx_seqspace: RxSequenceSpace,
    /// A quad containing remote and local destinations and ports.
    quad: ConnectionKey,
}

struct TxSequenceSpace {
    /// send unack'd
    una: u32,
    /// send next
    nxt: u32,
    /// send window
    wnd: u32,
    /// send up
    up: bool,
    /// segment seq number used for last window update
    wl1: usize,
    /// segment ack number used for last window update
    wl2: usize,
    /// initial send seq num.
    iss: u32,
}

struct RxSequenceSpace {
    /// receive next
    nxt: u32,
    /// receive window
    wnd: u16,
    /// receive urgent pointer
    up: bool,
    /// initial receive seq num
    irs: u32,
}

impl TcpConnection {
    pub fn accept(tcp: Tcp, ip: &Ipv4) -> Option<(Self, Tcp)> {
        // only SYN requests count as valid handshake packets.
        if !tcp.is_syn() {
            return None;
        }

        let this = Self {
            state: TcpStates::TCP_SYN_RECEIVED,
            tx_seqspace: TxSequenceSpace {
                iss: 0,
                una: 0,
                nxt: 1,
                wnd: 1024,
                up: false,
                wl1: 0,
                wl2: 0,
            },
            rx_seqspace: RxSequenceSpace {
                irs: tcp.seq_num(),
                nxt: tcp.seq_num() + 1,
                wnd: tcp.window(),
                up: false,
            },
            quad: (ip.sip(), tcp.src_port(), ip.dip(), tcp.dst_port()),
        };

        let mut packet = tcp.clone();

        packet.set_dst(this.quad.1);
        packet.set_src(this.quad.3);
        packet.set_flags(&[TcpFlag::SYN, TcpFlag::ACK]);
        packet.set_seq(this.tx_seqspace.nxt - 1); //replace this with a random num at runtime
        packet.set_ack(this.rx_seqspace.nxt);
        packet.set_checksum(ip.sip(), ip.dip());

        Some((this, packet))
    }

    pub fn handle_packet(&mut self, tcp: Tcp, ip: &Ipv4) -> Option<Tcp> {
        // check validity of the ack num
        let ackn = tcp.ack_num();

        if !(self.tx_seqspace.una.wrapping_sub(ackn) > (1 << 31)
            && ackn.wrapping_sub(self.tx_seqspace.nxt.wrapping_add(1)) > (1 << 31))
        {
            // received invalid ack
            if let TcpStates::TCP_SYN_RECEIVED | TcpStates::TCP_ESTABLISHED = self.state {
                return Some(self.reset(tcp, ip));
            }
            return None;
        }

        // Process ACKs
        if tcp.is_ack() {
            match self.state {
                TcpStates::TCP_SYN_RECEIVED => {
                    self.state = TcpStates::TCP_ESTABLISHED;
                }
                TcpStates::TCP_ESTABLISHED
                | TcpStates::TCP_FIN_WAIT_1
                | TcpStates::TCP_FIN_WAIT_2
                | TcpStates::TCP_CLOSE_WAIT
                | TcpStates::TCP_CLOSING
                | TcpStates::TCP_LAST_ACK => {
                    // TODO: Clean retransmission queue of packets that have been ack'd
                    if tcp.ack_num() < self.tx_seqspace.una {
                        // got a duplicate ack. safe to ignore
                        return None;
                    }

                    if tcp.ack_num() > self.tx_seqspace.nxt {
                        // if we got a ack for something we havent sent yet we just drop the packet
                        return None;
                    }
                }
                _ => {}
            }
        }

        // Process PSH
        if tcp.is_psh() {
            if let TcpStates::TCP_ESTABLISHED
            | TcpStates::TCP_FIN_WAIT_1
            | TcpStates::TCP_FIN_WAIT_2 = self.state
            {
                if tcp.seq_num() == self.rx_seqspace.nxt {
                    self.rx_seqspace.nxt += tcp.dlen() as u32;
                    println!("{}", String::from_utf8_lossy(tcp.data()));
                } else {
                    // TODO: Move this segment into a queue for later processing as it is within
                    // the window of data to receive but it is not the left most segment.
                    unimplemented!()
                }
            }
        }
        None
    }

    fn reset(&mut self, tcp: Tcp, ip: &Ipv4) -> Tcp {
        let mut packet = tcp.clone();
        packet.set_dst(self.quad.1);
        packet.set_src(self.quad.3);
        packet.clear_flags();
        packet.set_flags(&[TcpFlag::RST]);
        packet.set_seq(0);
        packet.set_ack(0);
        packet.set_checksum(ip.sip(), ip.dip());

        packet
    }
}
