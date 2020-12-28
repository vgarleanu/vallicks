use crate::net::frames::{ipaddr::Ipv4Addr, mac::Mac};
use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::{TryFrom, TryInto};

/// This struct holds the structure of ARP packets.
/// TODO: Unit tests
/// TODO: Enums where possible
#[derive(Debug, Clone)]
pub struct ArpPacket {
    /// Hardware type
    pub hardware_type: u16,
    /// Protocol
    pub protocol: u16,

    /// Hardware size
    pub hardware_size: u8,
    /// Protocol size
    pub protocol_size: u8,

    /// Opcode
    pub opcode: u16,

    /// Sender MAC address
    pub smac: Mac,
    /// Sender IP address
    pub sip: Ipv4Addr,
    /// Target MAC address
    pub tmac: Mac,
    /// Target IP address
    pub tip: Ipv4Addr,
}

impl TryFrom<&[u8]> for ArpPacket {
    type Error = TryFromSliceError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            hardware_type: u16::from_be_bytes(data[..2].try_into()?),
            protocol: u16::from_be_bytes(data[2..4].try_into()?),

            hardware_size: data[4],
            protocol_size: data[5],

            opcode: u16::from_be_bytes([data[6], data[7]]),

            smac: data[8..14].into(),
            sip: data[14..18].try_into()?,
            tmac: data[18..24].into(),
            tip: data[24..28].try_into()?,
        })
    }
}

impl TryFrom<Vec<u8>> for ArpPacket {
    type Error = TryFromSliceError;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self {
            hardware_type: u16::from_be_bytes(data[..2].try_into()?),
            protocol: u16::from_be_bytes(data[2..4].try_into()?),

            hardware_size: data[4],
            protocol_size: data[5],

            opcode: u16::from_be_bytes([data[6], data[7]]),

            smac: data[8..14].into(),
            sip: data[14..18].try_into()?,
            tmac: data[18..24].into(),
            tip: data[24..28].try_into()?,
        })
    }
}

impl Into<Vec<u8>> for ArpPacket {
    fn into(self) -> Vec<u8> {
        // TODO: Figure out if we can use slice patterns to make this nicer
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(&self.hardware_type.to_be_bytes());
        bytes.extend_from_slice(&self.protocol.to_be_bytes());
        bytes.extend_from_slice(&[self.hardware_size, self.protocol_size]);
        bytes.extend_from_slice(&self.opcode.to_be_bytes());

        bytes.extend_from_slice(self.smac.as_ref());
        bytes.extend_from_slice(self.sip.as_ref());
        bytes.extend_from_slice(self.tmac.as_ref());
        bytes.extend_from_slice(self.tip.as_ref());

        bytes
    }
}
