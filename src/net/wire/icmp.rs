use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::From;
use core::convert::Into;
use core::convert::TryFrom;
use core::convert::TryInto;
use core::mem::transmute;
use core::ops::{RangeFrom, RangeInclusive};

const ICMP_ECHO_MIN_SIZE: usize = 8;
const ICMP_ECHO_PACKET_TYPE: usize = 0;
const ICMP_ECHO_CODE: usize = 1;
const ICMP_ECHO_CSUM: RangeInclusive<usize> = 2..=3;
const ICMP_ECHO_IDENT: RangeInclusive<usize> = 4..=5;
const ICMP_ECHO_SEQ: RangeInclusive<usize> = 6..=7;
const ICMP_ECHO_DATA: RangeFrom<usize> = 8..;

#[derive(Clone, Debug)]
#[repr(u8)]
pub enum IcmpType {
    EchoReply = 0x00,
    Echo = 0x08,
}

impl IcmpType {
    pub fn raw(self) -> u8 {
        unsafe { transmute::<IcmpType, u8>(self) }
    }
}

impl From<u8> for IcmpType {
    fn from(i: u8) -> Self {
        unsafe { transmute::<u8, IcmpType>(i) }
    }
}

#[derive(Clone, Debug)]
#[repr(u8)]
pub enum IcmpCode {
    NetDown = 0x00,
    HostDown = 0x01,
    ProtocolDown = 0x02,
    PortDown = 0x03,
    FragNeeded = 0x04,
    SourceRouteFailed = 0x05,
    Unknown,
}

impl IcmpCode {
    pub fn raw(self) -> u8 {
        unsafe { transmute::<IcmpCode, u8>(self) }
    }
}

impl From<u8> for IcmpCode {
    fn from(i: u8) -> Self {
        unsafe { transmute::<u8, IcmpCode>(i) }
    }
}

/// Our basic ICMP packet struct.
/// TODO: Better packet structure docs.
#[derive(Clone)]
pub struct Icmp(Vec<u8>);

impl Icmp {
    pub fn packet_type(&self) -> IcmpType {
        self.0[ICMP_ECHO_PACKET_TYPE].into()
    }

    pub fn code(&self) -> IcmpCode {
        self.0[ICMP_ECHO_CODE].into()
    }

    pub fn checksum(&self) -> u16 {
        u16::from_be_bytes(
            self.0[ICMP_ECHO_CSUM]
                .try_into()
                .expect("net: icmp got no checksum"),
        )
    }

    pub fn identifier(&self) -> u16 {
        u16::from_be_bytes(
            self.0[ICMP_ECHO_IDENT]
                .try_into()
                .expect("net: icmp got no checksum"),
        )
    }

    pub fn seq(&self) -> u16 {
        u16::from_be_bytes(
            self.0[ICMP_ECHO_SEQ]
                .try_into()
                .expect("net: icmp got no checksum"),
        )
    }

    pub fn data(&self) -> &[u8] {
        &self.0[ICMP_ECHO_DATA]
    }

    pub fn set_packet_type(&mut self, packet_type: IcmpType) {
        self.0[ICMP_ECHO_PACKET_TYPE] = packet_type.raw();
    }

    pub fn set_code(&mut self, code: IcmpCode) {
        self.0[ICMP_ECHO_CODE] = code.raw();
    }

    pub fn set_checksum(&mut self) {
        // set it to 0
        self.0[ICMP_ECHO_CSUM].copy_from_slice(&0u16.to_le_bytes());

        let csum = crate::net::wire::ipv4::checksum(&self.0);
        self.0[ICMP_ECHO_CSUM].copy_from_slice(&csum.to_le_bytes());
    }

    pub fn set_identifier(&mut self, identifier: u16) {
        self.0[ICMP_ECHO_IDENT].copy_from_slice(&identifier.to_be_bytes());
    }

    pub fn set_seq(&mut self, seq: u16) {
        self.0[ICMP_ECHO_SEQ].copy_from_slice(&seq.to_be_bytes());
    }

    pub fn set_data<T: AsRef<[u8]>>(&mut self, data: T) {
        self.0.truncate(ICMP_ECHO_MIN_SIZE);
        self.0.extend_from_slice(data.as_ref());
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl super::Packet for Icmp {
    fn zeroed() -> Self {
        Self(vec![0; ICMP_ECHO_MIN_SIZE])
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, ()> {
        if bytes.len() < ICMP_ECHO_MIN_SIZE {
            return Err(());
        }

        Ok(Self(bytes))
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}
