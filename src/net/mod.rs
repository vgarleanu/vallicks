/// Our basic network stack
pub mod stack;
/// Our packet structures and parsers
pub mod wire;

pub use crate::net::wire as frames;
use alloc::vec::Vec;

/// Trait to be implemented by network device drivers.
///
/// This trait is essential for the network stack as it defines how raw packets can be send and
/// received from a NIC in a standard way.
pub trait PhyDevice {
    /// Function sends data down to the NIC. `data` will probably be encoded ether2 packets.
    fn send(&mut self, data: &[u8]) -> Result<usize, ()>;

    /// Function receives bytes from the NIC.
    fn recv(&mut self) -> Result<Option<Vec<u8>>, ()>;
}
