use crate::net::frames::ipaddr::Ipv4Addr;
use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::From;
use core::convert::Into;
use core::convert::TryFrom;
use core::convert::TryInto;

/// Our basic ICMP packet struct.
/// TODO: Better packet structure docs.
#[derive(Clone, Debug)]
pub struct Icmp {
    /// the Operation type of the ICMP packet.
    op_type: u8,
    /// The code of the packet.
    code: u8,
    /// The checksum of the packet.
    checksum: u16,
    /// The packet id in big endian form.
    id_be: u16,
    /// The packet id in little endian form.
    id_le: u16,
    /// The packet sequence number in big endian form.
    seq_be: u16,
    /// The packet sequence number in little endian form.
    seq_le: u16,
}

impl TryFrom<&[u8]> for Icmp {
    type Error = TryFromSliceError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        // We do this atm as we cant create a custom instance of TryFromSliceError
        if data.len() < 8 {
            let _: [u8; 8] = data.try_into()?;
        }

        Ok(Self {
            op_type: data[0],
            code: data[1],
            checksum: u16::from_be_bytes([data[2], data[3]]),
            id_be: u16::from_be_bytes([data[4], data[5]]),
            id_le: u16::from_le_bytes([data[4], data[5]]),
            seq_be: u16::from_be_bytes([data[6], data[7]]),
            seq_le: u16::from_le_bytes([data[6], data[7]]),
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
        let hdr: &[u8] = &[];

        let op = &[self.op_type, self.code];

        let checksum = &self.checksum.to_be_bytes()[..];
        let id = &self.id_be.to_be_bytes()[..];
        let seq = &self.seq_be.to_be_bytes()[..];

        [op, checksum, id, seq].join(hdr)
    }
}
