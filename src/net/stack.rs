use crate::driver::*;
use crate::net::ip::{Ether2Frame, Mac};
use crate::prelude::*;
use core::convert::TryInto;

#[derive(Clone, Copy)]
pub struct Ipv4Addr {
    inner: [u8; 4],
}

impl Ipv4Addr {
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            inner: [a, b, c, d],
        }
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            inner: data.try_into().unwrap(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_vec()
    }
}

impl core::fmt::Debug for Ipv4Addr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.inner[0], self.inner[1], self.inner[2], self.inner[3]
        )
    }
}

#[derive(Debug, Clone)]
struct ArpPacket {
    pub hardware_type: u16,
    pub protocol: u16,

    pub hardware_size: u8,
    pub protocol_size: u8,

    pub opcode: u16,

    pub smac: Mac,
    pub sip: Ipv4Addr,
    pub tmac: Mac,
    pub tip: Ipv4Addr,
}

impl ArpPacket {
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            hardware_type: u16::from_be_bytes(data[..2].try_into().unwrap()),
            protocol: u16::from_be_bytes(data[2..4].try_into().unwrap()),

            hardware_size: data[4],
            protocol_size: data[5],

            opcode: u16::from_be_bytes([data[6], data[7]]),

            smac: Mac::from_bytes(&data[8..14]),
            sip: Ipv4Addr::from_bytes(&data[14..18]),
            tmac: Mac::from_bytes(&data[18..24]),
            tip: Ipv4Addr::from_bytes(&data[24..28]),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // TODO: Figure out if we can use slice patterns to make this nicer
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(&self.hardware_type.to_be_bytes());
        bytes.extend_from_slice(&self.protocol.to_be_bytes());
        bytes.extend_from_slice(&[self.hardware_size, self.protocol_size]);
        bytes.extend_from_slice(&self.opcode.to_be_bytes());

        bytes.extend_from_slice(self.smac.to_bytes().as_ref());
        bytes.extend_from_slice(self.sip.to_bytes().as_ref());
        bytes.extend_from_slice(self.tmac.to_bytes().as_ref());
        bytes.extend_from_slice(self.tip.to_bytes().as_ref());

        bytes
    }
}

pub fn net_thread() {
    let mut lock = DRIVERS.lock();
    let ip = Ipv4Addr::new(192, 168, 100, 51);

    let mut driver = {
        lock.iter_mut()
            .filter_map(|x| {
                if let Driver::NetworkDriver(NetworkDriver::RTL8139(x)) = x {
                    Some(x)
                } else {
                    None
                }
            })
            .collect::<Vec<&mut rtl8139::RTL8139>>()
            .pop()
    }
    .expect("Unable to locate net driver");

    loop {
        if let Some(frame) = driver.try_read() {
            if frame.dtype() == 0x0806 {
                println!("{:?}", frame);
                let reply = handle_arp(frame, driver, ip);

                driver.write(reply.to_bytes().as_ref());
            }
        }

        thread::sleep(10); // sleep for 10 milis
    }
}

pub fn handle_arp(frame: Ether2Frame, driver: &rtl8139::RTL8139, ip: Ipv4Addr) -> Ether2Frame {
    let arp_frame = ArpPacket::from_bytes(frame.frame().as_ref());

    let mut reply = arp_frame.clone(); // We dont have to do much except swap shit around
    core::mem::swap(&mut reply.tmac, &mut reply.smac);

    reply.smac = driver.mac();
    reply.tip = arp_frame.sip;
    reply.sip = ip.clone();
    reply.opcode = 0x02; // ARP_REPLY TODO: Make this a global const

    Ether2Frame::new(arp_frame.smac, driver.mac(), 0x0806, reply.to_bytes())
}
