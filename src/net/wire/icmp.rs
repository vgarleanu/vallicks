use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::From;
use core::convert::Into;
use core::convert::TryFrom;
use core::convert::TryInto;
use core::mem::transmute;

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
#[derive(Clone, Debug)]
pub enum Icmp {
    Echo {
        packet_type: IcmpType,
        code: IcmpCode,
        checksum: u16,
        identifier: u16,
        sequence_number: u16,
        data: Vec<u8>,
    },
}

impl TryFrom<&[u8]> for Icmp {
    type Error = TryFromSliceError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        // We do this atm as we cant create a custom instance of TryFromSliceError
        if data.len() < 8 {
            panic!()
        }

        let op_type: IcmpType = data[0].into();

        Ok(match op_type {
            IcmpType::Echo | IcmpType::EchoReply => Self::Echo {
                packet_type: op_type,
                code: data[1].into(),
                checksum: u16::from_be_bytes([data[2], data[3]]),
                identifier: u16::from_be_bytes([data[4], data[5]]),
                sequence_number: u16::from_be_bytes([data[6], data[7]]),
                data: data[8..].to_vec(),
            },
        })
    }
}

impl TryFrom<Vec<u8>> for Icmp {
    type Error = TryFromSliceError;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        data.as_slice().try_into()
    }
}

impl Into<Vec<u8>> for Icmp {
    fn into(self) -> Vec<u8> {
        match self {
            Icmp::Echo {
                packet_type,
                code,
                checksum,
                identifier,
                sequence_number,
                data,
            } => {
                let hdr: &[u8] = &[];

                let op = &[packet_type.raw(), code.raw()];

                let checksum = &checksum.to_be_bytes()[..];
                let id = &identifier.to_be_bytes()[..];
                let seq = &sequence_number.to_be_bytes()[..];

                [op, checksum, id, seq, &data].join(hdr)
            }
        }
    }
}
