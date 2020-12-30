use crate::net::wire::Packet;
use crate::net::wire::{eth2::EtherType, ipaddr::Ipv4Addr, mac::Mac};
use crate::prelude::*;
use core::convert::TryInto;
use core::mem::transmute;
use core::ops::RangeInclusive;
use core::slice::SliceIndex;

const MIN_ARP_LEN: usize = 27; // the minimum valid length of a arp packet is 27b
const ARP_HW_TYPE: RangeInclusive<usize> = 0..=1;
const ARP_PROTO: RangeInclusive<usize> = 2..=3;
const ARP_HW_SIZE: usize = 4;
const ARP_PROTO_SIZE: usize = 5;
const ARP_OPCODE: RangeInclusive<usize> = 6..=7;
const ARP_SMAC: RangeInclusive<usize> = 8..=13;
const ARP_SIP: RangeInclusive<usize> = 14..=17;
const ARP_TMAC: RangeInclusive<usize> = 18..=23;
const ARP_TIP: RangeInclusive<usize> = 24..=27;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u16)]
pub enum ArpOpcode {
    ArpRequest,
    ArpReply = 2,
}

impl ArpOpcode {
    fn raw(self) -> u16 {
        self.into()
    }
}

impl Into<u16> for ArpOpcode {
    fn into(self) -> u16 {
        match self {
            Self::ArpRequest => 1,
            Self::ArpReply => 2,
        }
    }
}

impl From<u16> for ArpOpcode {
    fn from(data: u16) -> Self {
        unsafe { transmute::<u16, ArpOpcode>(data) }
    }
}

/// This struct holds the structure of ARP packets.
#[derive(Debug, Clone)]
pub struct ArpPacket(Vec<u8>);

// TODO: Make this generic to support `From::from` on `&mut [u8]` as well as `Vec<u8>`
impl ArpPacket {
    pub fn hw_type(&self) -> u16 {
        u16::from_be_bytes(self.0[ARP_HW_TYPE].try_into().expect("net: got no hw_type"))
    }

    pub fn proto(&self) -> EtherType {
        u16::from_be_bytes(self.0[ARP_HW_TYPE].try_into().expect("net: got no proto")).into()
    }

    pub fn hw_size(&self) -> u8 {
        self.0[ARP_HW_SIZE]
    }

    pub fn proto_size(&self) -> u8 {
        self.0[ARP_PROTO_SIZE]
    }

    pub fn opcode(&self) -> ArpOpcode {
        u16::from_be_bytes(self.0[ARP_OPCODE].try_into().expect("net: got no opcode")).into()
    }

    pub fn smac(&self) -> Mac {
        self.0[ARP_SMAC].into()
    }

    pub fn sip(&self) -> Ipv4Addr {
        self.0[ARP_SIP].try_into().expect("net: got no sip")
    }

    pub fn tmac(&self) -> Mac {
        self.0[ARP_TMAC].into()
    }

    pub fn tip(&self) -> Ipv4Addr {
        self.0[ARP_TIP].try_into().expect("net: got no tip")
    }

    pub fn set_hw_type(&mut self, hw_type: u16) {
        self.set(ARP_HW_TYPE, &hw_type.to_be_bytes());
    }

    pub fn set_proto(&mut self, proto: EtherType) {
        self.set(ARP_PROTO, &proto.raw().to_be_bytes());
    }

    pub fn set_hw_size(&mut self, hw_size: u8) {
        self.0[ARP_HW_SIZE] = hw_size;
    }

    pub fn set_proto_size(&mut self, proto_size: u8) {
        self.0[ARP_PROTO_SIZE] = proto_size;
    }

    pub fn set_opcode(&mut self, opcode: ArpOpcode) {
        self.set(ARP_OPCODE, &opcode.raw().to_be_bytes());
    }

    pub fn set_smac(&mut self, smac: Mac) {
        self.set(ARP_SMAC, &smac.as_ref());
    }

    pub fn set_sip(&mut self, sip: Ipv4Addr) {
        self.set(ARP_SIP, &sip.as_ref());
    }

    pub fn set_tmac(&mut self, tmac: Mac) {
        self.set(ARP_TMAC, &tmac.as_ref());
    }

    pub fn set_tip(&mut self, tip: Ipv4Addr) {
        self.set(ARP_TIP, &tip.as_ref());
    }

    fn set<T>(&mut self, range: T, data: &[u8])
    where
        T: SliceIndex<[u8]>,
        <T as SliceIndex<[u8]>>::Output: AsMut<[u8]>,
    {
        self.0[range].as_mut().copy_from_slice(data);
    }
}

impl super::Packet for ArpPacket {
    fn zeroed() -> Self {
        Self(vec![0; MIN_ARP_LEN])
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, ()> {
        if bytes.len() < MIN_ARP_LEN {
            return Err(());
        }

        Ok(Self(bytes))
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for ArpPacket {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
