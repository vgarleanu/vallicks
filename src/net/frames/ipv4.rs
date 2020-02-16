use crate::net::frames::ipaddr::Ipv4Addr;
use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::From;
use core::convert::Into;
use core::convert::TryFrom;
use core::convert::TryInto;

#[derive(Clone, Debug)]
pub struct Ipv4 {
    version: u8,
    hdr_len: u8,
    dscp: u8,
    ecn: u8,
    len: u16,
    id: u16,
    flags: u8,
    offset: u16,
    ttl: u8,
    proto: u8,
    checksum: u16,
    sip: Ipv4Addr,
    dip: Ipv4Addr,
}

impl TryFrom<&[u8]> for Ipv4 {
    type Error = TryFromSliceError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        // We do this atm as we cant create a custom instance of TryFromSliceError
        if data.len() < 20 {
            let _: [u8; 20] = data.try_into()?;
        }

        Ok(Self {
            version: data[0] >> 4,
            hdr_len: (data[0] & 0x0f) * 4,
            dscp: data[1] >> 2,

            ecn: data[1] & 0x03,

            len: u16::from_be_bytes([data[2], data[3]]),
            id: u16::from_be_bytes([data[4], data[5]]),

            flags: data[6] >> 5,
            offset: u16::from_be_bytes([data[6] & 0x1f, data[7]]),
            ttl: data[8],
            proto: data[9],

            checksum: u16::from_be_bytes([data[10], data[11]]),

            sip: data[12..16].try_into()?,
            dip: data[16..20].try_into()?,
        })
    }
}

impl TryFrom<Vec<u8>> for Ipv4 {
    type Error = TryFromSliceError;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
        data.as_slice().try_into()
    }
}

impl Into<Vec<u8>> for Ipv4 {
    fn into(self) -> Vec<u8> {
        let hdr: &[u8] = &[];

        let ver_dscp = &[
            // version and hdr_len bytes
            (self.version << 4) | (self.hdr_len / 4),
            (self.dscp << 2) | self.ecn,
        ];

        let len = &self.len.to_be_bytes();
        let id = &self.id.to_be_bytes();
        let flags = &(((self.flags as u16) << 8) | self.offset).to_be_bytes();
        let ttl_proto = &[self.ttl, self.proto][..];
        let checksum = &self.checksum.to_be_bytes();
        let sip = &self.sip.as_ref();
        let dip = &self.dip.as_ref();

        [ver_dscp, len, id, flags, ttl_proto, checksum, sip, dip].join(hdr)
    }
}
