//! Zero Copy Ethernet II packet parser.

use crate::net::frames::mac::Mac;
use crate::prelude::*;
use core::convert::{Into, TryInto};
use core::mem::transmute;
use core::ops::RangeFrom;
use core::ops::RangeInclusive;

const ETH2_MIN_VALID_SIZE: usize = 14;
/// Represents the range of 6 bytes pointing to the destination field.
const ETH2_DST_OFFSET: RangeInclusive<usize> = 0..=5;
/// Represents the range of 6 bytes pointing to the source field.
const ETH2_SRC_OFFSET: RangeInclusive<usize> = 6..=11;
/// Represents the range of bytes responsible for the dtype field.
const ETH2_DTYPE_OFFSET: RangeInclusive<usize> = 12..=13;
/// Represents the range of bytes responsible for the data field.
const ETH2_DATA_OFFSET: RangeFrom<usize> = 14..;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[repr(u16)]
pub enum EtherType {
    IPv4 = 0x0800,
    ARP = 0x0806,
    Ipv6 = 0x86dd,
    Unsupported,
}

impl EtherType {
    pub fn raw(self) -> u16 {
        unsafe { transmute::<EtherType, u16>(self) }
    }
}

impl From<u16> for EtherType {
    fn from(data: u16) -> Self {
        unsafe { transmute::<u16, EtherType>(data) }
    }
}

/// Structure represents a basic Ethernet II frame.
#[derive(Eq, PartialEq)]
pub struct Ether2Frame(Vec<u8>);

impl Ether2Frame {
    /// Returns the destination field value.
    pub fn dst(&self) -> Mac {
        self.0[ETH2_DST_OFFSET].into()
    }

    /// Returns the source of the packet.
    pub fn src(&self) -> Mac {
        self.0[ETH2_SRC_OFFSET].into()
    }

    /// Returns the dtype of the packet.
    pub fn dtype(&self) -> EtherType {
        u16::from_be_bytes(
            self.0[ETH2_DTYPE_OFFSET]
                .try_into()
                .expect("net: eth2 got null dtype"),
        )
        .into()
    }

    /// Returns a reference to the data in the packet.
    pub fn data(&self) -> &[u8] {
        &self.0[ETH2_DATA_OFFSET]
    }

    /// Sets the destination field value.
    pub fn set_dst(&mut self, dst: Mac) {
        self.0[ETH2_DST_OFFSET].copy_from_slice(dst.as_ref());
    }

    /// Sets the source field value.
    pub fn set_src(&mut self, src: Mac) {
        self.0[ETH2_SRC_OFFSET].copy_from_slice(src.as_ref());
    }

    /// Sets the dtype field.
    pub fn set_dtype(&mut self, dtype: EtherType) {
        self.0[ETH2_DTYPE_OFFSET].copy_from_slice(&dtype.raw().to_be_bytes());
    }

    /// Sets the data field.
    pub fn set_data<T: AsRef<[u8]>>(&mut self, data: T) {
        // remove the old data
        self.0.truncate(ETH2_MIN_VALID_SIZE);
        self.0.extend_from_slice(data.as_ref());
    }
}

impl super::Packet for Ether2Frame {
    fn zeroed() -> Self {
        Self(vec![0; ETH2_MIN_VALID_SIZE])
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self, ()> {
        if bytes.len() < ETH2_MIN_VALID_SIZE {
            return Err(());
        }

        Ok(Self(bytes))
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for Ether2Frame {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl core::fmt::Debug for Ether2Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Ether2Frame {{ dst: {}, src: {}, dtype: {:?}, frame: {:?} }}",
            self.dst(),
            self.src(),
            self.dtype(),
            self.data()
        )
    }
}
