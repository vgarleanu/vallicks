use crate::prelude::*;
use core::convert::TryInto;
use core::ops::RangeFrom;
use core::ops::RangeInclusive;

const TCP_MIN_LEN: usize = 24;
const TCP_SRC_PORT: RangeInclusive<usize> = 0..=1;
const TCP_DST_PORT: RangeInclusive<usize> = 2..=3;
const TCP_SEQ_NUM: RangeInclusive<usize> = 4..=7;
const TCP_ACK_NUM: RangeInclusive<usize> = 8..=11;
const TCP_DATA_OFFSET: usize = 12;
const TCP_FLAGS: usize = 13;
const TCP_WINDOW: RangeInclusive<usize> = 14..=15;
const TCP_CSUM: RangeInclusive<usize> = 16..=17;
const TCP_URGENT_PTR: RangeInclusive<usize> = 18..=19;
const TCP_OPTIONS: RangeInclusive<usize> = 20..=22;
const TCP_DATA: RangeFrom<usize> = 24..;

#[derive(Clone, Copy)]
pub enum TcpFlag {
    URG,
    ACK,
    PSH,
    RST,
    SYN,
    FIN,
}

impl Into<u8> for TcpFlag {
    fn into(self) -> u8 {
        match self {
            Self::URG => 1 << 5,
            Self::ACK => 1 << 4,
            Self::PSH => 1 << 3,
            Self::RST => 1 << 2,
            Self::SYN => 1 << 1,
            Self::FIN => 1,
        }
    }
}

#[derive(Clone)]
pub struct Tcp(Vec<u8>);

impl Tcp {
    pub fn src_port(&self) -> u16 {
        u16::from_be_bytes(
            self.0[TCP_SRC_PORT]
                .try_into()
                .expect("net: tcp got null src"),
        )
    }

    pub fn set_src(&mut self, src: u16) {
        self.0[TCP_SRC_PORT].copy_from_slice(&src.to_be_bytes());
    }

    pub fn dst_port(&self) -> u16 {
        u16::from_be_bytes(
            self.0[TCP_DST_PORT]
                .try_into()
                .expect("net: tcp got null dst"),
        )
    }

    pub fn set_dst(&mut self, dst: u16) {
        self.0[TCP_DST_PORT].copy_from_slice(&dst.to_be_bytes());
    }

    pub fn seq_num(&self) -> u32 {
        u32::from_be_bytes(
            self.0[TCP_SEQ_NUM]
                .try_into()
                .expect("net: tcp got null seq"),
        )
    }

    pub fn set_seq(&mut self, seq: u32) {
        self.0[TCP_SEQ_NUM].copy_from_slice(&seq.to_be_bytes())
    }

    pub fn ack_num(&self) -> u32 {
        u32::from_be_bytes(
            self.0[TCP_ACK_NUM]
                .try_into()
                .expect("net: tcp got null ack"),
        )
    }

    pub fn set_ack(&mut self, ack: u32) {
        self.0[TCP_ACK_NUM].copy_from_slice(&ack.to_be_bytes())
    }

    pub fn flaglist(&self) -> Vec<TcpFlag> {
        let mut flags = vec![];

        if self.is_urg() {
            flags.push(TcpFlag::URG);
        }

        if self.is_ack() {
            flags.push(TcpFlag::ACK);
        }

        if self.is_psh() {
            flags.push(TcpFlag::PSH);
        }

        if self.is_rst() {
            flags.push(TcpFlag::RST);
        }

        if self.is_syn() {
            flags.push(TcpFlag::SYN);
        }

        if self.is_fin() {
            flags.push(TcpFlag::FIN);
        }

        flags
    }

    pub fn flags(&self) -> u8 {
        self.0[TCP_FLAGS]
    }

    pub fn is_urg(&self) -> bool {
        self.flags() & (1 << 5) != 0
    }

    pub fn is_ack(&self) -> bool {
        self.flags() & (1 << 4) != 0
    }

    pub fn is_psh(&self) -> bool {
        self.flags() & (1 << 3) != 0
    }

    pub fn is_rst(&self) -> bool {
        self.flags() & (1 << 2) != 0
    }

    pub fn is_syn(&self) -> bool {
        self.flags() & (1 << 1) != 0
    }

    pub fn is_fin(&self) -> bool {
        self.flags() & 1 != 0
    }

    pub fn set_flags(&mut self, flags: &[TcpFlag]) {
        for i in flags {
            self.0[TCP_FLAGS] |= Into::<u8>::into(*i);
        }
    }

    pub fn window(&self) -> u16 {
        u16::from_be_bytes(
            self.0[TCP_WINDOW]
                .try_into()
                .expect("net: tcp got null window"),
        )
    }

    pub fn checksum(&self) -> u16 {
        u16::from_be_bytes(
            self.0[TCP_CSUM]
                .try_into()
                .expect("net: tcp got null checksum"),
        )
    }

    pub fn urgent_ptr(&self) -> u16 {
        u16::from_be_bytes(
            self.0[TCP_URGENT_PTR]
                .try_into()
                .expect("net: tcp got null urgent_ptr"),
        )
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl super::Packet for Tcp {
    fn zeroed() -> Self {
        Self(vec![0; TCP_MIN_LEN])
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, ()> {
        if bytes.len() < TCP_MIN_LEN {
            return Err(());
        }

        Ok(Self(bytes))
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl core::fmt::Debug for Tcp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut flags = vec![];
        if self.is_urg() {
            flags.push("URG");
        }

        if self.is_ack() {
            flags.push("ACK");
        }

        if self.is_psh() {
            flags.push("PSH");
        }

        if self.is_rst() {
            flags.push("RST");
        }

        if self.is_syn() {
            flags.push("SYN");
        }

        if self.is_fin() {
            flags.push("FIN");
        }

        write!(
            f,
            "Tcp {{ src: {}, dst: {}, seq: {}, ack: {}, window: {}, csum: {:#x}, uptr: {}, flags: {} }}",
            self.src_port(),
            self.dst_port(),
            self.seq_num(),
            self.ack_num(),
            self.window(),
            self.checksum(),
            self.urgent_ptr(),
            flags.join(","),
        )
    }
}
