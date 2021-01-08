//! Kernel drivers
//! This module is holds the basic kernel driver loading logic and structures. It is also home to
//! some default drivers are already implemented here and preloaded.
#![allow(missing_docs)]

use crate::arch::pci::Device;
use crate::net::wire::eth2::Ether2Frame;
use crate::net::wire::mac::Mac;
use crate::prelude::*;

use futures_util::sink::Sink;
use futures_util::stream::Stream;

pub mod keyboard;
pub mod rtl8139;
pub mod serial;
pub mod vga;

/// Trait marks a barebones implementation of a driver.
pub trait Driver {
    /// Type of the data we return at the end of init.
    type Return;

    /// Function must probe the hardware and find a device supported by this driver
    fn probe() -> Option<Device>;
    /// Function sets up and creates driver object without initializing the hardware.
    fn preload(device: Device) -> Self;
    /// Function initiates the hardware.
    fn init(&mut self) -> Self::Return;
}

/// Trait marks a network driver.
pub trait NetworkDriver: Driver + Send {
    /// Stream from where we can acquire ether2 frames.
    type RxSink: Stream<Item = Vec<u8>> + Send + Unpin;
    /// Stream over which we can send packets.
    type TxSink: Sink<Vec<u8>, Error = ()> + Send + Unpin;

    /// Splits the network driver into two separate sinks.
    fn parts(&mut self) -> (Self::RxSink, Self::TxSink);
    /// Returns the active mac address of this device.
    fn mac(&self) -> Mac;
}
