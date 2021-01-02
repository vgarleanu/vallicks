use crate::net::frames::ipaddr::Ipv4Addr;
use crate::prelude::*;
use core::convert::From;
use core::convert::Into;
use core::convert::TryInto;
use core::mem::transmute;
use core::ops::RangeFrom;
use core::ops::RangeInclusive;

const IPV4_MIN_VALID_LENGTH: usize = 20;
const IPV4_VERSION_OFFSET: usize = 0;
const IPV4_HDR_LEN_OFFSET: usize = 0;
const IPV4_DSCP_ECN: usize = 1;
const IPV4_LEN_OFFSET: RangeInclusive<usize> = 2..=3;
const IPV4_ID_OFFSET: RangeInclusive<usize> = 4..=5;
const IPV4_FLAGS_OFFSET: usize = 6;
const IPV4_OFFSET_OFFSET: RangeInclusive<usize> = 6..=7;
const IPV4_TTL_OFFSET: usize = 8;
const IPV4_PROTO_OFFSET: usize = 9;
const IPV4_CHECKSUM_OFFSET: RangeInclusive<usize> = 10..=11;
const IPV4_SIP_OFFSET: RangeInclusive<usize> = 12..=15;
const IPV4_DIP_OFFSET: RangeInclusive<usize> = 16..=19;
const IPV4_HEADER_OFFSET: RangeInclusive<usize> = 0..=19;
const IPV4_DATA_OFFSET: RangeFrom<usize> = 22..;

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Ipv4Proto {
    ICMP = 0x01,
    TCP = 0x06,
    UDP = 0x11,
    Unknown,
}

impl Ipv4Proto {
    pub fn raw(self) -> u8 {
        unsafe { transmute::<Ipv4Proto, u8>(self) }
    }
}

impl From<u8> for Ipv4Proto {
    fn from(i: u8) -> Self {
        unsafe { transmute::<u8, Ipv4Proto>(i) }
    }
}

/// The bare structure of Ipv4 packets
/// TODO: Use enums where possible
/// TODO: Unit tests
#[derive(Clone, Debug)]
pub struct Ipv4(Vec<u8>);

impl Ipv4 {
    pub fn set_version(&mut self, version: u8) {
        self.0[IPV4_VERSION_OFFSET] = version << 4 | (self.0[IPV4_HDR_LEN_OFFSET] & 0x0f);
    }

    pub fn set_hdr_len(&mut self, hdr_len: u8) {
        self.0[IPV4_HDR_LEN_OFFSET] |= hdr_len;
    }

    pub fn set_dscp_ecn(&mut self, dscp_ecn: u8) {
        self.0[IPV4_DSCP_ECN] = dscp_ecn;
    }

    pub fn set_len(&mut self) {
        let data_len = self.0[IPV4_DATA_OFFSET].len() as u16;
        let total_len = IPV4_MIN_VALID_LENGTH as u16 + data_len;
        self.0[IPV4_LEN_OFFSET].copy_from_slice(&total_len.to_be_bytes());
    }

    pub fn set_id(&mut self, id: u16) {
        self.0[IPV4_ID_OFFSET].copy_from_slice(&id.to_be_bytes());
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.0[IPV4_FLAGS_OFFSET] |= flags;
    }

    pub fn set_offset(&mut self, offset: u16) {
        let value = ((self.flags() as u16) << 8) | offset;
        self.0[IPV4_OFFSET_OFFSET].copy_from_slice(&value.to_be_bytes());
    }

    pub fn set_ttl(&mut self, ttl: u8) {
        self.0[IPV4_TTL_OFFSET] = ttl;
    }

    pub fn set_proto(&mut self, proto: Ipv4Proto) {
        self.0[IPV4_PROTO_OFFSET] = proto.raw();
    }

    pub fn set_checksum(&mut self) {
        let csum = u32_to_u16(checksum(&self.0[IPV4_HEADER_OFFSET]));
        self.0[IPV4_CHECKSUM_OFFSET].copy_from_slice(&csum.to_ne_bytes());
    }

    pub fn set_sip(&mut self, sip: Ipv4Addr) {
        self.0[IPV4_SIP_OFFSET].copy_from_slice(sip.as_ref());
    }

    pub fn set_dip(&mut self, dip: Ipv4Addr) {
        self.0[IPV4_DIP_OFFSET].copy_from_slice(dip.as_ref());
    }

    pub fn set_data<T: AsRef<[u8]>>(&mut self, data: T) {
        self.0.truncate(IPV4_MIN_VALID_LENGTH);
        self.0.extend_from_slice(data.as_ref());
    }

    pub fn version(&self) -> u8 {
        self.0[IPV4_VERSION_OFFSET] >> 4
    }

    pub fn hdr_len(&self) -> u8 {
        self.0[IPV4_VERSION_OFFSET] & 0x0f
    }

    pub fn dscp_ecn(&self) -> u8 {
        self.0[IPV4_DSCP_ECN]
    }

    pub fn len(&self) -> u16 {
        u16::from_be_bytes(
            self.0[IPV4_LEN_OFFSET]
                .try_into()
                .expect("net: ipv4 got no len"),
        )
    }

    pub fn id(&self) -> u16 {
        u16::from_be_bytes(
            self.0[IPV4_ID_OFFSET]
                .try_into()
                .expect("net: ipv4 got no id"),
        )
    }

    pub fn flags(&self) -> u8 {
        self.0[IPV4_FLAGS_OFFSET] >> 5
    }

    pub fn offset(&self) -> u16 {
        u16::from_be_bytes(
            self.0[IPV4_OFFSET_OFFSET]
                .try_into()
                .expect("net: ipv4 got no offset"),
        ) & 0x1fff
    }

    pub fn ttl(&self) -> u8 {
        self.0[IPV4_TTL_OFFSET]
    }

    pub fn proto(&self) -> Ipv4Proto {
        self.0[IPV4_PROTO_OFFSET].into()
    }

    pub fn checksum(&self) -> u16 {
        u16::from_be_bytes(
            self.0[IPV4_CHECKSUM_OFFSET]
                .try_into()
                .expect("net: ipv4 got no checksum"),
        )
    }

    pub fn sip(&self) -> Ipv4Addr {
        self.0[IPV4_SIP_OFFSET]
            .try_into()
            .expect("net: ipv4 got no sip")
    }

    pub fn dip(&self) -> Ipv4Addr {
        self.0[IPV4_DIP_OFFSET]
            .try_into()
            .expect("net: ipv4 got no dip")
    }

    pub fn data(&self) -> &[u8] {
        &self.0[IPV4_DATA_OFFSET]
    }

    pub fn header(&self) -> &[u8] {
        &self.0[IPV4_HEADER_OFFSET]
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl super::Packet for Ipv4 {
    fn zeroed() -> Self {
        let mut new_v4 = Self(vec![0; IPV4_MIN_VALID_LENGTH]);
        new_v4.set_version(4);
        new_v4.set_hdr_len(5);
        new_v4.set_flags(0x40);
        new_v4.set_ttl(64);
        new_v4.set_proto(Ipv4Proto::ICMP);
        new_v4.set_sip(Ipv4Addr::new(127, 0, 0, 1));
        new_v4.set_dip(Ipv4Addr::new(127, 0, 0, 1));

        new_v4
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, ()> {
        if bytes.len() < IPV4_MIN_VALID_LENGTH {
            return Err(());
        }

        let mut this = Self(bytes);
        this.0[IPV4_CHECKSUM_OFFSET].copy_from_slice(&[0, 0]);

        Ok(this)
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

pub fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0;

    let mut data_iter = data.chunks_exact(2);

    while let Some(x) = data_iter.next() {
        sum += u16::from_le_bytes(x.try_into().unwrap()) as u32;
    }

    if let [item, ..] = data_iter.remainder() {
        sum += *item as u32;
    }

    sum
}

pub fn u32_to_u16(mut sum: u32) -> u16 {
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    (!sum & 0xffff) as u16
}
