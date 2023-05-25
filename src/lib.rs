#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;
pub mod bin;
mod util;

#[cfg(feature = "alloc")]
use alloc::{
    borrow::{Cow, ToOwned},
    collections::BTreeMap,
};
#[cfg(not(feature = "alloc"))]
pub use bin::ResTblReader;
use thiserror_no_std::Error;
use util::String;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Insufficient data: expected {0}")]
    InsufficientData(&'static str),
    #[error("Invalid magic: {0:?}, expected \"RESTBL\"")]
    InvalidMagic([u8; 6]),
    #[error("Invalid table size: {0}, expected {1}")]
    InvalidTableSize(usize, usize),
    #[error(transparent)]
    Utf8Error(#[from] core::str::Utf8Error),
}

pub enum TableIndex<'a> {
    HashIndex(u32),
    #[cfg(feature = "alloc")]
    StringIndex(Cow<'a, str>),
    #[cfg(not(feature = "alloc"))]
    StringIndex(&'a str),
}

impl From<u32> for TableIndex<'_> {
    fn from(value: u32) -> Self {
        TableIndex::HashIndex(value)
    }
}

impl<'a> From<&'a str> for TableIndex<'a> {
    fn from(value: &'a str) -> Self {
        #[cfg(feature = "alloc")]
        {
            TableIndex::StringIndex(value.into())
        }
        #[cfg(not(feature = "alloc"))]
        {
            TableIndex::StringIndex(value)
        }
    }
}

impl<'a> From<&'a String> for TableIndex<'a> {
    fn from(value: &'a String) -> Self {
        #[cfg(feature = "alloc")]
        {
            TableIndex::StringIndex(Cow::Borrowed(value.as_str()))
        }
        #[cfg(not(feature = "alloc"))]
        {
            TableIndex::StringIndex(value.as_str())
        }
    }
}

#[cfg(feature = "alloc")]
impl From<String> for TableIndex<'_> {
    fn from(value: String) -> Self {
        TableIndex::StringIndex(Cow::Owned(value.as_str().to_owned()))
    }
}

#[cfg(feature = "alloc")]
impl From<alloc::string::String> for TableIndex<'_> {
    fn from(value: alloc::string::String) -> Self {
        TableIndex::StringIndex(value.into())
    }
}

#[cfg(feature = "alloc")]
pub struct ResourceSizeTable {
    pub crc_table: BTreeMap<u32, u32>,
    pub string_table: BTreeMap<String, u32>,
}
