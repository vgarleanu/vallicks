use core::convert::{AsRef, From, TryInto};
use core::hash::{Hash, Hasher};

/// Represents a MAC address
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Mac {
    /// The inner bytes of our mac address.
    inner: [u8; 6],
}

impl Mac {
    pub fn multicast() -> Self {
        Self {
            inner: [0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        }
    }
}

impl From<&[u8]> for Mac {
    fn from(data: &[u8]) -> Self {
        Self {
            inner: data[..6].try_into().expect("Got mac longer than expected"),
        }
    }
}

impl From<[u8; 6]> for Mac {
    fn from(data: [u8; 6]) -> Self {
        Self { inner: data }
    }
}

impl AsRef<[u8]> for Mac {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

impl core::fmt::Display for Mac {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let m = self.inner;
        write!(
            f,
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            m[0], m[1], m[2], m[3], m[4], m[5]
        )
    }
}

impl Hash for Mac {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}
