use super::wire::eth2::Ether2Frame;
use super::wire::eth2::EtherType;
use super::wire::ipaddr::Ipv4Addr;
use super::wire::ipv4::Ipv4;
use super::wire::ipv4::Ipv4Proto;
use super::wire::mac::Mac;
use super::wire::tcp::Tcp;
use super::wire::tcp::TcpFlag;
use super::wire::tcp::TcpStates;
use super::wire::Packet;

use crate::prelude::*;
use crate::sync::mpsc::UnboundedReceiver;
use crate::sync::mpsc::UnboundedSender;
use crate::sync::RwLock;
use crate::net::socks::TcpStream;
use crate::sync::Mutex;

use core::task::Waker;

use alloc::sync::Arc;
use hashbrown::HashMap;
use hashbrown::hash_map::Entry;

pub type ConnectionKey = (Ipv4Addr, u16, Ipv4Addr, u16); // sip, sport, dip, dport
pub type ConnectionMap = HashMap<ConnectionKey, Arc<Mutex<TcpConnection>>>;

pub struct TcpLayer {
    connections: RwLock<ConnectionMap>,
}

impl TcpLayer {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(ConnectionMap::new()),
        }
    }

    pub async fn handle_packet(&self, packet: Tcp, ctx: &Ipv4) -> Option<Tcp> {
        let conn_key = (ctx.sip(), packet.src(), ctx.dip(), packet.dst());

        let local_mac = super::ARP_LAYER.resolve_ip_local(ctx.dip()).await.expect("failed to get local mac");
        let mac = super::ARP_LAYER.resolve_ip(ctx.sip()).await.expect("failed to resolve remote ip");

        match self.connections.write().await.entry(conn_key) {
            Entry::Occupied(mut entry) => {
                let mut lock = entry.get_mut().lock().await;
                return lock.handle_packet(packet, ctx);
            }
            Entry::Vacant(entry) => {
                let key = packet.dst();
                let lock = super::OPEN_PORTS.read();

                // we are listening on dst port
                if let Some(listener) = lock.get(&key) {
                    match TcpConnection::accept(
                        packet,
                        ctx,
                        local_mac,
                        mac,
                    ) {
                        Ok((conn, out)) => {
                            let conn = Arc::new(crate::sync::Mutex::new(conn));
                            let stream = TcpStream { raw: conn.clone() };

                            listener
                                .send(stream)
                                .expect("failed to send key to listener");
                            entry.insert(conn);

                            return Some(out);
                        }
                        Err(e) => return e,
                    }
                }
                return None;
            }
        }
    }

    pub async fn handle_tx(&self, packet: Tcp, sip: Ipv4Addr, dip: Ipv4Addr) {
        super::IP_LAYER.handle_tx(packet.as_bytes(), Ipv4Proto::TCP, dip, sip).await;
    }
}

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
    snd_wl1: u32,
    /// segment ack number used for last window update
    snd_wl2: u32,
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
    /// bytes received so far
    data: Vec<u8>,
    /// Waker for task waiting on data.
    waker: Option<Waker>,
    /// Last ipv4 packet id
    last_ipv4_id: u16,
    /// Mac of this device.
    mac: Mac,
    /// Mac of the outbound device,
    dst_mac: Mac,
}

impl TcpConnection {
    pub fn accept(
        tcp: Tcp,
        ip: &Ipv4,
        mac: Mac,
        dst_mac: Mac,
    ) -> Result<(Self, Tcp), Option<Tcp>> {
        // First check for a RST
        if tcp.is_rst() {
            return Err(None);
        }

        // if we get an ack we must send a RST RFC793 p.65
        if tcp.is_ack() {
            let mut packet = Tcp::zeroed();
            packet.set_dst(tcp.src());
            packet.set_src(tcp.dst());
            packet.set_flags(&[TcpFlag::RST]);
            packet.set_seq(tcp.ack());
            packet.set_hlen(20);
            packet.set_checksum(ip.sip(), ip.dip());

            return Err(Some(packet));
        }

        // only SYN requests count as valid handshake packets.
        if !tcp.is_syn() {
            return Err(None);
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
            rcv_irs: tcp.seq(),
            rcv_nxt: tcp.seq() + 1,
            rcv_wnd: tcp.window(),
            rcv_up: false,
            quad: (ip.sip(), tcp.src(), ip.dip(), tcp.dst()),
            data: Vec::new(),
            waker: None,
            last_ipv4_id: ip.id(),
            mac,
            dst_mac,
        };

        let mut packet = Tcp::zeroed();

        packet.set_dst(this.quad.1);
        packet.set_src(this.quad.3);
        packet.set_flags(&[TcpFlag::SYN, TcpFlag::ACK]);
        packet.set_seq(this.snd_iss); //replace this with a random num at runtime
        packet.set_ack(this.rcv_nxt);
        packet.set_hlen(20);
        packet.set_checksum(ip.sip(), ip.dip());

        Ok((this, packet))
    }

    pub fn handle_packet(&mut self, tcp: Tcp, ip: &Ipv4) -> Option<Tcp> {
        // handle keep_alives
        if let TcpStates::TCP_ESTABLISHED = self.state {
            if tcp.is_ack() && !tcp.is_psh() {
                return Some(self.ack(tcp, ip));
            }
        }

        // SYN-SENT state
        if let TcpStates::TCP_SYNSENT = self.state {
            if tcp.is_ack() {
                if tcp.ack() <= self.snd_iss || self.snd_nxt < tcp.ack() {
                    // return Some(self.reset(tcp, ip)); // <SEQ=SEG.ACK>
                    return None;
                }

                // use wrapping comparations
                if self.snd_una <= tcp.ack() && tcp.ack() <= self.snd_nxt {
                    if tcp.is_rst() {
                        // TODO: Drop segment and close connection
                        return None;
                    }
                }
            }

            // 4th step, check the syn bit
            if tcp.is_syn() {
                self.rcv_nxt = tcp.seq() + 1;
                self.rcv_irs = tcp.seq();

                // TODO: SND.UNA should be advanced to equal SEG.ACK (if there
                // is an ACK), and any segments on the retransmission queue which
                // are thereby acknowledged should be removed
                if tcp.is_ack() {
                    self.snd_una += 1
                }

                // our SYN has been ack'd
                if self.snd_una > self.snd_iss {
                    self.state = TcpStates::TCP_ESTABLISHED;
                    return Some(self.ack(tcp, ip)); // <SEQ=SND.NXT><ACK=RCV.NXT><CTL=ACK>
                }
            }

            if !tcp.is_syn() || !tcp.is_rst() {
                return None;
            }
        }
        // TODO: p.69 check the seq number again??
        // if invalid <SEQ=SND.NXT><ACK=RCV.NXT><CTL=ACK>

        // second check the rst bit p.70 RFC793
        if tcp.is_rst() {
            match self.state {
                TcpStates::TCP_SYN_RECEIVED => {
                    // If this connection was initiated with a passive OPEN (i.e.,
                    // came from the LISTEN state), then return this connection to
                    // LISTEN state and return.  The user need not be informed.  If
                    // this connection was initiated with an active OPEN (i.e., came
                    // from SYN-SENT state) then the connection was refused, signal
                    // the user "connection refused".  In either case, all segments
                    // on the retransmission queue should be removed.  And in the
                    // active OPEN case, enter the CLOSED state and delete the TCB,
                    // and return.

                    // TODO: Remove this TCP connection from the tcp stack as it is marked CLOSED.
                    self.state = TcpStates::TCP_CLOSE;
                }
                TcpStates::TCP_ESTABLISHED
                | TcpStates::TCP_FIN_WAIT_1
                | TcpStates::TCP_FIN_WAIT_2
                | TcpStates::TCP_CLOSE_WAIT => {
                    // If the RST bit is set then, any outstanding RECEIVEs and SEND
                    // should receive "reset" responses.  All segment queues should be
                    // flushed.  Users should also receive an unsolicited general
                    // "connection reset" signal.  Enter the CLOSED state, delete the
                    // TCB, and return.
                    self.state = TcpStates::TCP_CLOSE;
                }
                TcpStates::TCP_CLOSING | TcpStates::TCP_LAST_ACK | TcpStates::TCP_TIME_WAIT => {
                    // If the RST bit is set then, enter the CLOSED state, delete the
                    // TCB, and return.
                    self.state = TcpStates::TCP_CLOSE;
                }
                TcpStates::TCP_SYNSENT | TcpStates::TCP_LISTEN | TcpStates::TCP_CLOSE => {
                    println!(
                        "tcp: attempted to process packet when socket is SYNSENT | LISTEN | CLOSE"
                    );
                }
            }
            return None;
        }

        // fourth check syn bit p.71
        if tcp.is_syn() {
            // If the SYN is in the window it is an error, send a reset, any
            // outstanding RECEIVEs and SEND should receive "reset" responses,
            // all segment queues should be flushed, the user should also
            // receive an unsolicited general "connection reset" signal, enter
            // the CLOSED state, delete the TCB, and return.

            // If the SYN is not in the window this step would not be reached
            // and an ack would have been sent in the first step (sequence
            // number check).
            //
            // NOTE: I think its safe to assume that we can just reset any connection if
            // this branch is reached.

            self.state = TcpStates::TCP_CLOSE;
            return Some(self.reset(tcp, ip));
        }

        // fifth check the ack field
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
                    if self.snd_una < tcp.ack() && tcp.ack() <= self.snd_nxt {
                        self.snd_una = tcp.ack();
                        // TODO: clean retransmission queue here and send acks to our clients
                        // waiting for confirmation of send's

                        if self.snd_wl1 < tcp.seq()
                            || (self.snd_wl1 == tcp.seq() && self.snd_wl2 <= tcp.ack())
                        {
                            self.snd_wnd = tcp.window() as u32;
                            self.snd_wl1 = tcp.seq();
                            self.snd_wl2 = tcp.ack();
                        }

                        // FIN-WAIT-1 STATE
                        if let TcpStates::TCP_FIN_WAIT_1 = self.state {
                            // NOTE: Do we have to do extra checking of the packet to ensure that
                            // this ack ack's our FIN?
                            self.state = TcpStates::TCP_FIN_WAIT_2;
                        }

                        // FIN-WAIT-2 STATE
                        if let TcpStates::TCP_FIN_WAIT_2 = self.state {
                            // If the retransmission queue is empty the users CLOSE can be ok'd
                            // without deleting the TCB.
                        }

                        // CLOSING STATE
                        if let TcpStates::TCP_CLOSING = self.state {
                            // In addition to the processing for the ESTABLISHED state, if
                            // the ACK acknowledges our FIN then enter the TIME-WAIT state,
                            // otherwise ignore the segment.
                        }

                        // LAST-ACK STATE
                        if let TcpStates::TCP_LAST_ACK = self.state {
                            // The only thing that can arrive in this state is an
                            // acknowledgment of our FIN.  If our FIN is now acknowledged,
                            // delete the TCB, enter the CLOSED state, and return.
                        }

                        // TIME-WAIT STATE
                        if let TcpStates::TCP_TIME_WAIT = self.state {
                            // The only thing that can arrive in this state is a
                            // retransmission of the remote FIN.  Acknowledge it, and restart
                            // the 2 MSL timeout.
                        }
                    }
                }
                _ => {}
            }
        }

        // sixth, check the urg bit.
        if tcp.is_urg() {
            unimplemented!("Fuck you, this rfc is deprecated");
        }

        // seventh process segment text.
        if tcp.data().len() > 0 {
            if let TcpStates::TCP_ESTABLISHED
            | TcpStates::TCP_FIN_WAIT_1
            | TcpStates::TCP_FIN_WAIT_2 = self.state
            {
                if tcp.seq() == self.rcv_nxt {
                    // Once the TCP takes responsibility for the data it advances
                    // RCV.NXT over the data accepted, and adjusts RCV.WND as
                    // apporopriate to the current buffer availability.  The total of
                    // RCV.NXT and RCV.WND should not be reduced.
                    self.data.extend_from_slice(tcp.data());

                    self.rcv_nxt += tcp.dlen() as u32;

                    // wake the async read task.
                    if let Some(waker) = self.waker.take() {
                        waker.wake();
                    }

                    return Some(self.ack(tcp, ip)); // send our ack
                } else {
                    // TODO: Move this segment into a queue for later processing as it is within
                    // the window of data to receive but it is not the left most segment.
                    unimplemented!()
                }
            }
        }

        // eighth check the fin bit.
        if tcp.is_fin() {
            match self.state {
                TcpStates::TCP_CLOSE | TcpStates::TCP_LISTEN | TcpStates::TCP_SYNSENT => {
                    // dont progress segment
                    return None;
                }
                TcpStates::TCP_SYN_RECEIVED | TcpStates::TCP_ESTABLISHED => {
                    self.state = TcpStates::TCP_CLOSE_WAIT;
                }
                TcpStates::TCP_FIN_WAIT_1 => {
                    // If our FIN has been ACKed (perhaps in this segment), then
                    // enter TIME-WAIT, start the time-wait timer, turn off the other
                    // timers; otherwise enter the CLOSING state.
                    self.state = TcpStates::TCP_FIN_WAIT_2;
                }
                TcpStates::TCP_TIME_WAIT => {
                    // TODO: Restart 2msl time wait timeout.
                }
                _ => {}
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

    pub async fn write(&mut self, item: &[u8]) {
        self.last_ipv4_id += 1;

        let mut packet = Tcp::zeroed();
        packet.set_dst(self.quad.1);
        packet.set_src(self.quad.3);
        packet.set_flags(&[TcpFlag::PSH, TcpFlag::ACK]);
        packet.set_seq(self.snd_nxt);
        packet.set_ack(self.rcv_nxt);
        packet.set_window(self.rcv_wnd);
        packet.set_hlen(20);
        packet.set_data(item.to_vec());
        packet.set_checksum(self.quad.0, self.quad.2);

        self.snd_nxt += item.len() as u32;

        super::TCP_LAYER.handle_tx(packet, self.quad.2, self.quad.0).await;
    }

    pub fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    pub fn register_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    /// Function reads data into a buffer returning the number of bytes read.
    pub fn read(&mut self, buffer: &mut [u8]) -> usize {
        let min_len = buffer.len().min(self.data.len());
        buffer[..min_len].copy_from_slice(&self.data[..min_len]);
        self.data.drain(..min_len);
        min_len
    }
}
