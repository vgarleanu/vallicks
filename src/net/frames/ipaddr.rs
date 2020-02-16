use core::array::TryFromSliceError;
use core::convert::{AsRef,  TryFrom, TryInto};

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
}

impl TryFrom<&[u8]> for Ipv4Addr {
    type Error = TryFromSliceError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            inner: data.try_into()?,
        })
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
