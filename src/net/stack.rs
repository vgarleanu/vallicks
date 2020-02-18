use crate::driver::*;
use crate::net::frames::{
    arp::ArpPacket, eth2::Ether2Frame, icmp::Icmp, ipaddr::Ipv4Addr, ipv4::Ipv4, mac::Mac,
};
use crate::naked_std::thread;
use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::TryInto;

/// TODO: FINISH THE NET STACK BEFORE DOCS
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
            println!("{}", frame.dtype());
            if frame.dtype() == 0x0806 {
                let reply = handle_arp(frame, driver, ip);

                driver.write(Into::<Vec<u8>>::into(reply).as_ref());
            }

            if frame.dtype() == 0x0800 {
                let ipv4: Ipv4 = frame.frame().try_into().unwrap();

                if ipv4.proto() == 0x01 {
                    let icmp: Icmp = ipv4.data().try_into().unwrap();

                    println!("{:#?}", icmp);
                }
            }
        }

        // FIXME: For some reason a context switch fails to restore %rax, causing lock xadd
        //        %rcx,0x10(%rax) to cause a double fault
        thread::sleep(1); // sleep for 10 milis
    }
}

/// Methos responds to a arp ping
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

/// Method responds to a icmp ping
pub fn handle_icmp(frame: &Ether2Frame, driver: &rtl8139::RTL8139, ip: Ipv4Addr) {
    let ipv4: Ipv4 = frame.frame().try_into().unwrap();
    println!("{:#?}", ipv4);
}
