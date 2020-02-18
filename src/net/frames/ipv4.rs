use crate::net::frames::ipaddr::Ipv4Addr;
use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::From;
use core::convert::Into;
use core::convert::TryFrom;
use core::convert::TryInto;

/// The bare structure of Ipv4 packets
/// TODO: Use enums where possible
/// TODO: Unit tests
#[derive(Clone, Debug)]
pub struct Ipv4 {
    /// Version of the packet, can be 4 or 6
    version: u8,
    /// The total length of the header
    hdr_len: u8,
    /// Dunno
    dscp: u8,
    /// Dunno
    ecn: u8,
    /// Total length??
    len: u16,
    /// ID of the packet
    id: u16,
    /// Flags for the packet
    flags: u8,
    /// Offset
    offset: u16,
    /// Time to live for the packet
    ttl: u8,
    /// Protocol ID
    proto: u8,
    /// Packet checksum
    checksum: u16,
    /// Send IP
    sip: Ipv4Addr,
    /// Destination IP
    dip: Ipv4Addr,

    /// Data extracted after the packet
    data: Vec<u8>,
}

impl Ipv4 {
    /// Method retruns the length of the packet.
    pub fn len(&self) -> u16 {
        self.len
    }

    /// Method returns the protocol id for this packet.
    pub fn proto(&self) -> u8 {
        self.proto
    }

    /// Method returns the data extracted after the packet
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
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

            data: data[20..].to_vec(),
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
