#[allow(unused_imports)]
use crate::prelude::*;
use core::convert::TryInto;

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Mac {
    m0: u8,
    m1: u8,
    m2: u8,
    m3: u8,
    m4: u8,
    m5: u8,
}

impl Mac {
    pub fn from_bytes(data: &[u8]) -> Self {
        Self {
            m0: data[0],
            m1: data[1],
            m2: data[2],
            m3: data[3],
            m4: data[4],
            m5: data[5],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        vec![self.m0, self.m1, self.m2, self.m3, self.m4, self.m5]
    }
}

impl core::fmt::Display for Mac {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            self.m0, self.m1, self.m2, self.m3, self.m4, self.m5
        )
    }
}

pub struct Ether2Frame {
    dst: Mac,
    src: Mac,
    dtype: u16,
    frame: Vec<u8>,
}

impl Ether2Frame {
    pub fn new(dst: Mac, src: Mac, dtype: u16, frame: Vec<u8>) -> Self {
        Self {
            dst,
            src,
            dtype,
            frame,
        }
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        if data.len() < 10 {
            panic!("Received a ip frame with length < 10");
        }

        Self {
            dst: Mac::from_bytes(&data[..6]),
            src: Mac::from_bytes(&data[6..12]),
            dtype: u16::from_be_bytes(data[12..14].try_into().expect("Ether2Frame: Invalid dtype")),
            frame: data[14..].to_vec(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // TODO: Figure out if we can use slice patterns to make this nicer
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(self.dst.to_bytes().as_ref());
        bytes.extend_from_slice(self.src.to_bytes().as_ref());
        bytes.extend_from_slice(self.dtype.to_be_bytes().as_ref());
        bytes.extend_from_slice(self.frame.as_ref());

        bytes
    }

    pub fn dtype(&self) -> u16 {
        self.dtype
    }

    pub fn frame(&self) -> Vec<u8> {
        self.frame.clone()
    }
}

impl core::fmt::Debug for Ether2Frame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Ether2Frame {{ dst: {}, src: {}, dtype: {:#x}, frame: {:?} }}",
            self.dst, self.src, self.dtype, self.frame
        )
    }
}
