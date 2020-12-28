use core::array::TryFromSliceError;
use core::convert::{AsRef, From, TryFrom, TryInto};

/// Struct represents a IP version 4 address
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct Ipv4Addr {
    /// Inner bytes of the IP address
    inner: [u8; 4],
}

impl Ipv4Addr {
    /// Method constructs a new IP from the given levels.
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            inner: [a, b, c, d],
        }
    }
}

impl TryFrom<&[u8]> for Ipv4Addr {
    type Error = TryFromSliceError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: data.try_into()?,
        })
    }
}

impl From<[u8; 4]> for Ipv4Addr {
    fn from(data: [u8; 4]) -> Self {
        Self { inner: data }
    }
}

impl AsRef<[u8]> for Ipv4Addr {
    fn as_ref(&self) -> &[u8] {
        &self.inner
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
