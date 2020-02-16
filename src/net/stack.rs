use crate::driver::*;
use crate::net::frames::arp::ArpPacket;
use crate::net::frames::eth2::Ether2Frame;
use crate::net::frames::ipaddr::Ipv4Addr;
use crate::net::frames::mac::Mac;
use crate::prelude::*;
use core::convert::TryInto;

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
        if let Some(ref frame) = driver.try_read() {
            println!("{:x}", frame.dtype());
            if frame.dtype() == 0x0806 {
                let reply = handle_arp(frame, driver, ip);

                driver.write(Into::<Vec<u8>>::into(reply).as_ref());
            }

            if frame.dtype() == 0x0800 {
                handle_icmp(frame, driver, ip);
            }
        }

        thread::sleep(10); // sleep for 10 milis
    }
}

pub fn handle_arp(frame: &Ether2Frame, driver: &rtl8139::RTL8139, ip: Ipv4Addr) -> Ether2Frame {
    let arp_frame: ArpPacket = frame.frame().try_into().unwrap();

    let mut reply = arp_frame.clone(); // We dont have to do much except swap shit around
    core::mem::swap(&mut reply.tmac, &mut reply.smac);

    reply.smac = driver.mac();
    reply.tip = arp_frame.sip;
    reply.sip = ip.clone();
    reply.opcode = 0x02; // ARP_REPLY TODO: Make this a global const

    Ether2Frame::new(arp_frame.smac, driver.mac(), 0x0806, reply.into())
}

pub fn handle_icmp(frame: &Ether2Frame, driver: &rtl8139::RTL8139, ip: Ipv4Addr) {
    println!("ICMP");
}
