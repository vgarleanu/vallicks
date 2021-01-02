use super::wire::ipaddr::Ipv4Addr;
use super::wire::ipv4::Ipv4;
use super::wire::tcp::Tcp;
use super::wire::tcp::TcpFlag;
use super::wire::tcp::TcpStates;
use super::wire::Packet;

use crate::prelude::*;
use hashbrown::HashMap;

pub type ConnectionKey = (Ipv4Addr, u16, Ipv4Addr, u16); // sip, sport, dip, dport
pub type ConnectionMap = HashMap<ConnectionKey, TcpConnection>;

pub struct TcpConnection {
    /// Current state of this tcp connection
    state: TcpStates,
    /// A quad containing remote and local destinations and ports.
    quad: ConnectionKey,
    /// send unack'd
    snd_una: u32,
    /// send next
    snd_nxt: u32,
    /// send window
    snd_wnd: u32,
    /// send up
    snd_up: bool,
    /// segment seq number used for last window update
    snd_wl1: usize,
    /// segment ack number used for last window update
    snd_wl2: usize,
    /// initial send seq num.
    snd_iss: u32,
    /// receive next
    rcv_nxt: u32,
    /// receive window (essentially how many bytes at once we want to receive)
    rcv_wnd: u16,
    /// receive urgent pointer
    rcv_up: bool,
    /// initial receive seq num
    rcv_irs: u32,
}

impl TcpConnection {
    pub fn accept(tcp: Tcp, ip: &Ipv4) -> Option<(Self, Tcp)> {
        // only SYN requests count as valid handshake packets.
        if !tcp.is_syn() {
            return None;
        }

        let this = Self {
            state: TcpStates::TCP_SYN_RECEIVED,
            snd_iss: 0,
            snd_una: 0,
            snd_nxt: 1,
            snd_wnd: 1024,
            snd_up: false,
            snd_wl1: 0,
            snd_wl2: 0,
            rcv_irs: tcp.seq_num(),
            rcv_nxt: tcp.seq_num() + 1,
            rcv_wnd: tcp.window(),
            rcv_up: false,
            quad: (ip.sip(), tcp.src_port(), ip.dip(), tcp.dst_port()),
        };

        let mut packet = tcp.clone();

        packet.set_dst(this.quad.1);
        packet.set_src(this.quad.3);
        packet.set_flags(&[TcpFlag::SYN, TcpFlag::ACK]);
        packet.set_seq(this.snd_nxt); //replace this with a random num at runtime
        packet.set_ack(this.rcv_nxt);
        packet.set_checksum(ip.sip(), ip.dip());

        Some((this, packet))
    }

    pub fn handle_packet(&mut self, tcp: Tcp, ip: &Ipv4) -> Option<Tcp> {
        // check validity of the ack num
        let ackn = tcp.ack_num();

        if !(self.snd_una.wrapping_sub(ackn) > (1 << 31)
            && ackn.wrapping_sub(self.snd_nxt.wrapping_add(1)) > (1 << 31))
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
                    if tcp.ack_num() < self.snd_una {
                        // got a duplicate ack. safe to ignore
                        return None;
                    }

                    if tcp.ack_num() > self.snd_nxt {
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
                if tcp.seq_num() == self.rcv_nxt {
                    self.rcv_nxt += tcp.len() as u32;
                    println!("{}", String::from_utf8_lossy(tcp.data()));
                    return Some(self.ack(tcp, ip)); // send our ack
                } else {
                    // TODO: Move this segment into a queue for later processing as it is within
                    // the window of data to receive but it is not the left most segment.
                    unimplemented!()
                }
            }
        }
        None
    }

    fn ack(&mut self, tcp: Tcp, ip: &Ipv4) -> Tcp {
        let mut packet = Tcp::zeroed();
        packet.set_flags(&[TcpFlag::ACK]);
        packet.set_dst(self.quad.1);
        packet.set_src(self.quad.3);
        packet.set_hlen(20);
        packet.set_seq(self.snd_nxt);
        packet.set_ack(self.rcv_nxt);
        packet.set_window(self.rcv_wnd);
        packet.set_checksum(ip.sip(), ip.dip());

        packet
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

    fn write(&mut self, mut tcp: Tcp, ip: &Ipv4, seq: u32) -> Tcp {
        tcp.set_seq(seq);
        tcp.set_ack(self.rcv_nxt);

        let mut offset = seq.wrapping_sub(self.snd_una) as usize;
        todo!()
    }
}
