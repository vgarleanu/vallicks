/// Holds our ARP packet structure and parser.
pub mod arp;
/// Holds our Ethernet II packet structure and parser.
pub mod eth2;
/// Holds our ICMP packet structure and parser.
pub mod icmp;
/// Holds our IpAddr structure and parser.
pub mod ipaddr;
/// Holds our IPv4 packet structure and parser.
pub mod ipv4;
/// Holds our MAC address structure and parser.
pub mod mac;
/// Holds our TCP packet structures.
pub mod tcp;
// pub mod udp;

use crate::prelude::Vec;

/// Marks a packet.
pub trait Packet: Sized {
    /// Create a new packet that is zeroed out.
    fn zeroed() -> Self;
    /// Parse a stream of bytes and construct a packet.
    /// Ideally this methid should return `Err(())` if
    /// the packet is corrupted or invalid in any form.
    fn from_bytes(bytes: Vec<u8>) -> Result<Self, ()>;
    /// Converts this packet into a vector of bytes that are ready to be merged to the data section
    /// of other packets or ready to be sent down to the network driver.
    fn into_bytes(self) -> Vec<u8>;
}

impl Packet for () {
    fn zeroed() -> () {
        ()
    }

    fn from_bytes(_: Vec<u8>) -> Result<(), ()> {
        Err(())
    }

    fn into_bytes(self) -> Vec<u8> {
        Vec::new()
    }
}
