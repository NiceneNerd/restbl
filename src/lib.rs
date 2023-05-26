#![cfg_attr(not(any(feature = "std", test)), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;
pub mod bin;
#[cfg(feature = "yaml")]
mod text;
mod util;

#[cfg(feature = "alloc")]
use alloc::{
    borrow::{Cow, ToOwned},
    collections::BTreeMap,
};
#[cfg(not(feature = "alloc"))]
pub use bin::ResTblReader;
use thiserror_no_std::Error;
use util::Name;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Insufficient data: found {0} bytes, expected {1}")]
    InsufficientData(usize, &'static str),
    #[error("Invalid magic: {0:?}, expected \"RESTBL\"")]
    InvalidMagic([u8; 6]),
    #[error("Invalid table size: {0}, expected {1}")]
    InvalidTableSize(usize, usize),
    #[error(transparent)]
    Utf8Error(#[from] core::str::Utf8Error),
    #[error("Buffer too small for output: found {0} bytes, requires at least {1}")]
    InsufficientBuffer(usize, usize),
    #[cfg(feature = "std")]
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
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

impl<'a> From<&'a Name> for TableIndex<'a> {
    fn from(value: &'a Name) -> Self {
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
impl From<Name> for TableIndex<'_> {
    fn from(value: Name) -> Self {
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceSizeTable {
    pub crc_table: BTreeMap<u32, u32>,
    pub name_table: BTreeMap<Name, u32>,
}

#[cfg(feature = "alloc")]
impl ResourceSizeTable {
    pub fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        fn inner(data: &[u8]) -> Result<ResourceSizeTable> {
            let parser = bin::ResTblReader::new(data)?;
            let mut crc_table = BTreeMap::new();
            let mut name_table = BTreeMap::new();
            for entry in parser.iter() {
                match entry {
                    bin::TableEntry::Hash(entry) => crc_table.insert(entry.hash(), entry.value()),
                    bin::TableEntry::Name(entry) => name_table.insert(entry.name(), entry.value()),
                };
            }
            Ok(ResourceSizeTable {
                crc_table,
                name_table,
            })
        }
        inner(data.as_ref())
    }
}

#[cfg(test)]
mod test {
    pub(crate) static DATA: &[u8] =
        include_bytes!("../test/ResourceSizeTable.Product.110.rsizetable");
}
