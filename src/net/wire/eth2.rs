use crate::net::frames::mac::Mac;
use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::{Into, TryFrom, TryInto};
use core::mem::transmute;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[repr(u16)]
pub enum EtherType {
    IPv4 = 0x0800,
    ARP = 0x0806,
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
#[derive(Eq, PartialEq, Clone)]
pub struct Ether2Frame {
    /// Destination MAC address
    dst: Mac,
    /// Source MAC Address
    src: Mac,
    /// Data type
    dtype: EtherType,
    /// Frame extracted.
    frame: Vec<u8>,
}

impl Ether2Frame {
    /// Creates a new bare Eth2 frame from the given parameters
    ///
    /// # Arguments
    /// * `dst` - The destination for this packet
    /// * `src` - The source for this packet
    /// * `dtype` - The data type for the frame
    /// * `frame` - The actual frame to send downstream
    pub fn new(dst: Mac, src: Mac, dtype: EtherType, frame: Vec<u8>) -> Self {
        Self {
            dst,
            src,
            dtype,
            frame,
        }
    }

    /// Returns the dtype of this frame.
    // TODO: Return enum instead
    pub fn dtype(&self) -> EtherType {
        self.dtype
    }

    /// Returns the dtype of this frame raw
    pub fn dtype_raw(&self) -> u16 {
        unsafe { transmute::<EtherType, u16>(self.dtype) }
    }

    pub fn dst(&self) -> Mac {
        self.dst
    }

    /// Returns the entire frame.
    pub fn frame(&self) -> Vec<u8> {
        self.frame.clone()
    }
}

impl TryFrom<&[u8]> for Ether2Frame {
    type Error = TryFromSliceError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            dst: data[..6].into(),
            src: data[6..12].into(),
            dtype: unsafe {
                transmute::<u16, EtherType>(u16::from_be_bytes(data[12..14].try_into()?))
            },
            frame: data[14..].to_vec(),
        })
    }
}

impl Into<Vec<u8>> for Ether2Frame {
    fn into(self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(self.dst.as_ref());
        bytes.extend_from_slice(self.src.as_ref());
        bytes.extend_from_slice(self.dtype_raw().to_be_bytes().as_ref());
        bytes.extend_from_slice(self.frame.as_ref());

        bytes
    }
}

impl core::fmt::Debug for Ether2Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Ether2Frame {{ dst: {}, src: {}, dtype: {:?}, frame: {:?} }}",
            self.dst, self.src, self.dtype, self.frame
        )
    }
}
