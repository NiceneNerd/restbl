//! # restbl
//!
//! A simple library to handle RSTB/RESTBL (resource size table) files from *The
//! Legend of Zelda: Tears of the Kingdom. Features:
//! - Quick, zero-allocation parser
//! - Optional `alloc` feature to support editable table which can be serialized to
//!   binary or (with the `yaml` feature) YAML.
//! - `no_std` support (optional `std` feature)
//! - optional Serde support (`serde` feature)
//! - `aarch64-nintendo-switch-freestanding` support (without the `std` feature)
//!
//! ## Example Usage
//!
//! ```rust
//! use restbl::bin::ResTblReader;
//!
//! let bytes = std::fs::read("test/ResourceSizeTable.Product.110.rsizetable").unwrap();
//!
//! // Setup the quick, zero-allocation reader
//! let reader = ResTblReader::new(bytes.as_slice()).unwrap();
//! // Lookup an RSTB value
//! assert_eq!(
//!     reader.get("Bake/Scene/MainField_G_26_43.bkres"),
//!     Some(31880)
//! );
//!
//! #[cfg(feature = "alloc")]
//! {
//!     use restbl::ResourceSizeTable;
//!     // Parse RSTB into owned table
//!     let mut table = ResourceSizeTable::from_parser(&reader);
//!     // Set the size for a resource
//!     table.set("TexToGo/Etc_BaseCampWallWood_A_Alb.txtg", 777);
//!     // Check the size
//!     assert_eq!(
//!         table.get("TexToGo/Etc_BaseCampWallWood_A_Alb.txtg"),
//!         Some(777)
//!     );
//!     // Dump to YAML, if `yaml` feature enabled
//!     #[cfg(feature = "yaml")]
//!     {
//!         let json_table = table.to_text();
//!         // From YAML back to RSTB
//!         let new_table = ResourceSizeTable::from_text(&json_table).unwrap();
//!     }
//! }
//! ```
//!
//! ## Building for Switch
//!
//! To build for Switch, you will need to use the
//! `aarch64-nintendo-switch-freestanding` target. The `std` feature is not
//! supported, so you will need to use `--no-default-features`. Since [`cargo
//! nx`](https://github.com/aarch64-switch-rs/cargo-nx) does not seem to support
//! passing feature flags, you will need to run the full command yourself, as
//! follows:
//!
//! > cargo build -Z build-std=core,compiler_builtins,alloc --target aarch64-nintendo-switch-freestanding --no-default-features
//!
//! ## License
//!
//! This software is licensed under the terms of the GNU General Public License,
//! version 3 or later.
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

/// Result type for this create
pub type Result<T> = core::result::Result<T, Error>;

/// Error type for this crate
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
    #[cfg(all(feature = "alloc", feature = "yaml"))]
    #[error("Invalid YAML line: {0}")]
    YamlError(alloc::string::String),
    #[cfg(feature = "yaml")]
    #[error("Invalid number in YAML line: {0}")]
    YamlInvalidNumber(#[from] core::num::ParseIntError),
}

/// Represents an index into the RSTB, which can be a canonical resource path or
/// its hash
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

/// Data structure representing Tears of the Kingdom's resource size table
/// (`ResourceSizeTable.Product.rsizetable.zs`). Requires the `alloc` feature.
/// Can be serialized or deserialized to binary or (with the `text` feature) a
/// YAML document.
#[cfg(feature = "alloc")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ResourceSizeTable {
    pub crc_table: BTreeMap<u32, u32>,
    pub name_table: BTreeMap<Name, u32>,
}

#[cfg(feature = "alloc")]
impl ResourceSizeTable {
    /// Construct an empty table
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct an owned table from a fast readonly parser
    pub fn from_parser(parser: &bin::ResTblReader<'_>) -> Self {
        let mut crc_table = BTreeMap::new();
        let mut name_table = BTreeMap::new();
        for entry in parser.iter() {
            match entry {
                bin::TableEntry::Hash(entry) => crc_table.insert(entry.hash(), entry.value()),
                bin::TableEntry::Name(entry) => name_table.insert(entry.name(), entry.value()),
            };
        }
        ResourceSizeTable {
            crc_table,
            name_table,
        }
    }

    /// Get the total number of hash and name entries in the table
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.crc_table.len() + self.name_table.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if the specified hash or resource name is present in the table.
    /// Checks the name table first (if applicable) and then the hash table.
    pub fn contains<'i, I: Into<TableIndex<'i>>>(&self, needle: I) -> bool {
        fn inner(tbl: &ResourceSizeTable, needle: TableIndex) -> bool {
            match needle {
                TableIndex::HashIndex(hash) => tbl.crc_table.contains_key(&hash),
                TableIndex::StringIndex(name) => {
                    tbl.name_table.contains_key(&Name::from(name.as_ref())) || {
                        let hash = util::hash_name(&name);
                        tbl.crc_table.contains_key(&hash)
                    }
                }
            }
        }
        inner(self, needle.into())
    }

    /// Returns the RSTB value for the specified hash or resource name if
    /// present. Checks the name table first (if applicable) and then the hash
    /// table.
    pub fn get<'i, I: Into<TableIndex<'i>>>(&self, needle: I) -> Option<u32> {
        fn inner(tbl: &ResourceSizeTable, needle: TableIndex) -> Option<u32> {
            match needle {
                TableIndex::HashIndex(hash) => tbl.crc_table.get(&hash),
                TableIndex::StringIndex(name) => {
                    tbl.name_table.get(&Name::from(name.as_ref())).or_else(|| {
                        let hash = util::hash_name(&name);
                        tbl.crc_table.get(&hash)
                    })
                }
            }
            .copied()
        }
        inner(self, needle.into())
    }

    /// Returns a mutable reference to the RSTB value for the specified hash or
    /// resource name if present. Checks the name table first (if applicable)
    /// and then the hash table.
    pub fn get_mut<'i, I: Into<TableIndex<'i>>>(&mut self, needle: I) -> Option<&mut u32> {
        fn inner<'a>(
            tbl: &'a mut ResourceSizeTable,
            needle: TableIndex<'_>,
        ) -> Option<&'a mut u32> {
            match needle {
                TableIndex::HashIndex(hash) => tbl.crc_table.get_mut(&hash),
                TableIndex::StringIndex(name) => tbl
                    .name_table
                    .get_mut(&Name::from(name.as_ref()))
                    .or_else(|| {
                        let hash = util::hash_name(&name);
                        tbl.crc_table.get_mut(&hash)
                    }),
            }
        }
        inner(self, needle.into())
    }

    /// Set the RSTB value for the specified hash or resource name, returning
    /// the original value if present. Checks the name table first (if
    /// applicable) and then the hash table.
    pub fn set<'i, I: Into<TableIndex<'i>>>(&mut self, res: I, value: u32) -> Option<u32> {
        fn inner(tbl: &mut ResourceSizeTable, needle: TableIndex, value: u32) -> Option<u32> {
            match needle {
                TableIndex::HashIndex(hash) => tbl.crc_table.insert(hash, value),
                TableIndex::StringIndex(name) => {
                    match tbl.name_table.entry(Name::from(name.as_ref())) {
                        alloc::collections::btree_map::Entry::Occupied(mut e) => {
                            Some(e.insert(value))
                        }
                        alloc::collections::btree_map::Entry::Vacant(_) => {
                            let hash = util::hash_name(&name);
                            tbl.crc_table.insert(hash, value)
                        }
                    }
                }
            }
        }
        inner(self, res.into(), value)
    }

    /// Remove the RSTB value for the specified hash or resource name, returning
    /// the original value if present. Checks the name table first (if
    /// applicable) and then the hash table.
    pub fn remove<'i, I: Into<TableIndex<'i>>>(&mut self, res: I) -> Option<u32> {
        fn inner(tbl: &mut ResourceSizeTable, needle: TableIndex<'_>) -> Option<u32> {
            match needle {
                TableIndex::HashIndex(hash) => tbl.crc_table.remove(&hash),
                TableIndex::StringIndex(name) => tbl
                    .name_table
                    .remove(&Name::from(name.as_ref()))
                    .or_else(|| {
                        let hash = util::hash_name(&name);
                        tbl.crc_table.remove(&hash)
                    }),
            }
        }
        inner(self, res.into())
    }

    /// Set multiple RSTB entries from an iterator
    pub fn extend<'i, N: Into<TableIndex<'i>>, I: Iterator<Item = (N, u32)>>(&mut self, iter: I) {
        fn inner<'i, I: Iterator<Item = (TableIndex<'i>, u32)>>(
            tbl: &mut ResourceSizeTable,
            iter: I,
        ) {
            for (k, v) in iter {
                tbl.set(k, v);
            }
        }
        inner(self, iter.map(|(k, v)| (k.into(), v)))
    }
}

#[cfg(test)]
mod test {
    pub(crate) static DATA: &[u8] =
        include_bytes!("../test/ResourceSizeTable.Product.110.rsizetable");
}
