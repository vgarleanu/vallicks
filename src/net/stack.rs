use crate::driver::*;
use crate::net::frames::{
    arp::ArpPacket, eth2::Ether2Frame, icmp::Icmp, ipaddr::Ipv4Addr, ipv4::Ipv4, mac::Mac,
};
use crate::prelude::*;
use core::array::TryFromSliceError;
use core::convert::TryInto;

/// TODO: FINISH THE NET STACK BEFORE DOCS
pub fn net_thread() {
    /*
    loop {
        if let Some(ref frame) = driver.try_read() {
            println!("{}", frame.dtype());
            if frame.dtype() == 0x0800 {
                let ipv4: Ipv4 = frame.frame().try_into().unwrap();

                if ipv4.proto() == 0x01 {
                    let icmp: Icmp = ipv4.data().try_into().unwrap();

                    println!("{:#?}", icmp);
                }
            }
        }
    }
    */
}

/// Method responds to a icmp ping
pub fn handle_icmp(frame: &Ether2Frame, driver: &rtl8139::RTL8139, ip: Ipv4Addr) {
    let ipv4: Ipv4 = frame.frame().try_into().unwrap();
    println!("{:#?}", ipv4);
}
