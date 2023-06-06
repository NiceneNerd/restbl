#![allow(clippy::unused_unit)]
#![cfg_attr(not(feature = "alloc"), allow(clippy::needless_borrow))]
#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
use core::mem::size_of;
use memoffset::offset_of;
use sa::static_assert;

use crate::{
    util::{hash_name, read_u32, Name},
    Error, Result, TableIndex,
};

/// Constant representing the magic of an RESTBL file
pub const MAGIC: &[u8] = b"RESTBL";

/// Represents an RESTBL header, without the magic
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Header {
    version: u32,
    string_block_size: u32,
    crc_table_count: u32,
    name_table_count: u32,
}
static_assert!(Header::FULL_SIZE == 0x16);

impl Header {
    const FULL_SIZE: usize = size_of::<Header>() + MAGIC.len();

    #[inline(always)]
    pub fn version(&self) -> u32 {
        self.version
    }

    #[inline(always)]
    pub fn string_block_size(&self) -> u32 {
        self.string_block_size
    }

    #[inline(always)]
    pub fn crc_table_count(&self) -> u32 {
        self.crc_table_count
    }

    #[inline(always)]
    pub fn name_table_count(&self) -> u32 {
        self.name_table_count
    }

    /// Attempt to parse the RESTBL header from a slice
    fn read(data: &[u8]) -> Result<Self> {
        if data.len() < Self::FULL_SIZE {
            Err(Error::InsufficientData(data.len(), "0x16 bytes for header"))
        } else if &data[..MAGIC.len()] != MAGIC {
            Err(Error::InvalidMagic(
                data[..MAGIC.len()]
                    .try_into()
                    .expect("Slice must be 6 bytes long"),
            ))
        } else {
            let data = &data[MAGIC.len()..Self::FULL_SIZE];
            Ok(Self {
                version: read_u32(data, None)?,
                string_block_size: read_u32(data, Some(offset_of!(Header, string_block_size)))?,
                crc_table_count: read_u32(data, Some(offset_of!(Header, crc_table_count)))?,
                name_table_count: read_u32(data, Some(offset_of!(Header, name_table_count)))?,
            })
        }
    }

    /// Attempt to serialize the RESTBL header to a writer
    pub fn write(self, buffer: &mut [u8]) {
        buffer[..MAGIC.len()].copy_from_slice(MAGIC);
        buffer[MAGIC.len()..Self::FULL_SIZE].copy_from_slice(
            unsafe { core::mem::transmute::<Self, [u8; core::mem::size_of::<Self>()]>(self) }
                .as_slice(),
        )
    }
}

/// Represents a RESTBL hash entry
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashEntry {
    hash: u32,
    value: u32,
}
static_assert!(size_of::<HashEntry>() == 0x8);

impl HashEntry {
    /// Attempt to parse a RESTBL hash entry from a slice
    pub fn read(buffer: &[u8]) -> Result<Self> {
        if buffer.len() < size_of::<HashEntry>() {
            Err(Error::InsufficientData(
                buffer.len(),
                "8 bytes for HashEntry",
            ))
        } else {
            Ok(Self {
                hash: read_u32(buffer, None)?,
                value: read_u32(buffer, Some(offset_of!(HashEntry, value)))?,
            })
        }
    }

    /// Serialize a RESTBL hash entry to a buffer
    pub fn write(self, buffer: &mut [u8]) {
        buffer[..size_of::<Self>()].copy_from_slice(
            unsafe { core::mem::transmute::<Self, [u8; core::mem::size_of::<Self>()]>(self) }
                .as_slice(),
        );
    }

    #[inline(always)]
    pub fn hash(&self) -> u32 {
        self.hash
    }

    #[inline(always)]
    pub fn value(&self) -> u32 {
        self.value
    }
}

/// Represents a RESTBL name entry
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameEntry {
    name: Name,
    value: u32,
}
static_assert!(size_of::<NameEntry>() == 0xa4);

impl NameEntry {
    /// Attempt to parse a RESTBL name entry from a slice
    pub fn read(buffer: &[u8]) -> Result<Self> {
        if buffer.len() < size_of::<NameEntry>() {
            Err(Error::InsufficientData(
                buffer.len(),
                "0x4a bytes for NameEntry",
            ))
        } else {
            Ok(Self {
                name: Name::try_from(&buffer[..160])?,
                value: read_u32(buffer, Some(160))?,
            })
        }
    }

    /// Serialize a RESTBL name entry to a buffer
    pub fn write(self, buffer: &mut [u8]) {
        buffer[..size_of::<Self>()].copy_from_slice(
            unsafe { core::mem::transmute::<Self, [u8; core::mem::size_of::<Self>()]>(self) }
                .as_slice(),
        );
    }

    #[inline(always)]
    pub fn name(&self) -> Name {
        self.name
    }

    #[inline(always)]
    pub fn value(&self) -> u32 {
        self.value
    }
}

/// An extremely fast, non-allocating reader to parse an RSTB file
pub struct ResTblReader<'a> {
    #[cfg(feature = "alloc")]
    data: Cow<'a, [u8]>,
    #[cfg(not(feature = "alloc"))]
    data: &'a [u8],
    header: Header,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TableEntry {
    Hash(HashEntry),
    Name(NameEntry),
}

struct HashTableIndex(usize);
struct NameTableIndex(usize);

/// Struct to iterate over RSTB entries
pub struct ResTblIterator<'a> {
    table: &'a ResTblReader<'a>,
    index: usize,
}

impl<'a> Iterator for ResTblIterator<'a> {
    type Item = TableEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(hash_index) = self.table.hash_table_index(self.index) {
            let entry = self.table.parse_hash_entry(hash_index);
            self.index += 1;
            Some(TableEntry::Hash(entry))
        } else {
            let start = self.table.name_table_offset()
                + (self.index - self.table.header.crc_table_count as usize)
                    * size_of::<NameEntry>();
            let end = start + size_of::<NameEntry>();
            if end > self.table.data.len() {
                None
            } else {
                let data = &self.table.data[start..start + size_of::<NameEntry>()];
                let entry = NameEntry::read(data).ok();
                self.index += 1;
                entry.map(TableEntry::Name)
            }
        }
    }
}

#[cfg(feature = "alloc")]
type Buffer<'a> = alloc::borrow::Cow<'a, [u8]>;
#[cfg(not(feature = "alloc"))]
type Buffer<'a> = &'a [u8];

impl<'a> ResTblReader<'a> {
    /// Construct a new RSTB parser
    pub fn new<D: Into<Buffer<'a>>>(data: D) -> Result<Self> {
        fn inner(data: Buffer<'_>) -> Result<ResTblReader<'_>> {
            let header = Header::read(&data[..Header::FULL_SIZE])?;
            let expected_size = Header::FULL_SIZE
                + header.crc_table_count as usize * size_of::<HashEntry>()
                + header.name_table_count as usize * size_of::<NameEntry>();
            if data.len() < expected_size {
                Err(Error::InvalidTableSize(data.len(), expected_size))
            } else {
                Ok(ResTblReader { data, header })
            }
        }
        inner(data.into())
    }

    #[inline(always)]
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Get the total number of hash and name entries in the table
    #[inline(always)]
    pub fn len(&self) -> usize {
        (self.header.crc_table_count + self.header.name_table_count) as usize
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// SAFETY: This involves two unsafe operations, `core::mem::transmute` and
    /// unchecked slice-to-array. They are perfectly sound, however. The slice
    /// conversion is sound because the size of the slice and the size of the
    /// array are both set specifically by the size of `HashEntry`, the slice is
    /// guaranteed to be within bounds because the table size was checked in the
    /// `new()` method, the only way to construct this parser, and the index
    /// type is only ever constructed if within the bounds of the hash table. The
    /// transmute is sound because any possible combination of 8 bytes can be
    /// legitimately interpreted as a pair of `u32` values. The values could be
    /// nonsense if the file is not valid, but they cannot produce undefined
    /// behavior.
    fn parse_hash_entry(&self, index: HashTableIndex) -> HashEntry {
        debug_assert!(index.0 < self.header.crc_table_count as usize);
        let start = Header::FULL_SIZE + index.0 * size_of::<HashEntry>();
        let end = start + size_of::<HashEntry>();
        unsafe {
            core::mem::transmute::<[u8; size_of::<HashEntry>()], HashEntry>(
                self.data
                    .get_unchecked(start..end)
                    .try_into()
                    .unwrap_unchecked(),
            )
        }
    }

    /// Check if the specified hash or resource name is present in the table.
    /// Checks the name table first (if applicable) and then the hash table.
    pub fn contains<'i, I: Into<TableIndex<'i>>>(&self, needle: I) -> bool {
        fn inner(tbl: &ResTblReader, needle: TableIndex) -> bool {
            match needle {
                TableIndex::HashIndex(hash) => tbl.find_hash_entry(hash).is_some(),
                TableIndex::StringIndex(name) => {
                    tbl.find_name_entry(&name).map(|e| e.value).is_some() || {
                        let hash = hash_name(&name);
                        tbl.find_hash_entry(hash).map(|e| e.value).is_some()
                    }
                }
            }
        }
        inner(self, needle.into())
    }

    /// Returns the RSTB value for the specified hash or resource name if present.
    /// Checks the name table first (if applicable) and then the hash table.
    pub fn get<'i, I: Into<TableIndex<'i>>>(&self, needle: I) -> Option<u32> {
        fn inner(tbl: &ResTblReader, needle: TableIndex) -> Option<u32> {
            match needle {
                TableIndex::HashIndex(hash) => tbl.find_hash_entry(hash).map(|e| e.value),
                TableIndex::StringIndex(name) => {
                    tbl.find_name_entry(&name).map(|e| e.value).or_else(|| {
                        let hash = hash_name(&name);
                        tbl.find_hash_entry(hash).map(|e| e.value)
                    })
                }
            }
        }
        inner(self, needle.into())
    }

    fn parse_name_entry(&self, index: NameTableIndex) -> Result<NameEntry> {
        let start = self.name_table_offset() + index.0 * size_of::<NameEntry>();
        let end = start + size_of::<NameEntry>();
        NameEntry::read(&self.data[start..end])
    }

    fn find_hash_entry(&self, hash: u32) -> Option<HashEntry> {
        let mut start = 0;
        let mut end = self.header.crc_table_count as usize;
        while start < end {
            let mid = (start + end) / 2;
            let entry = self.parse_hash_entry(HashTableIndex(mid));
            match entry.hash.cmp(&hash) {
                core::cmp::Ordering::Less => {
                    start = mid + 1;
                }
                core::cmp::Ordering::Greater => {
                    end = mid;
                }
                core::cmp::Ordering::Equal => return Some(entry),
            }
        }
        None
    }

    fn find_name_entry(&self, name: &str) -> Option<NameEntry> {
        let mut start = 0;
        let mut end = self.header.name_table_count as usize;
        while start < end {
            let mid = (start + end) / 2;
            let entry = self.parse_name_entry(NameTableIndex(mid)).ok()?;
            match entry.name.partial_cmp(&name) {
                Some(core::cmp::Ordering::Less) => {
                    start = mid + 1;
                }
                Some(core::cmp::Ordering::Greater) => {
                    end = mid;
                }
                Some(core::cmp::Ordering::Equal) => return Some(entry),
                _ => return None,
            }
        }
        None
    }

    /// Returns the RSTB entry for the specified hash or resource name if present.
    /// Checks the name table first (if applicable) and then the hash table.
    pub fn get_entry<'i, I: Into<TableIndex<'i>>>(&self, needle: I) -> Option<TableEntry> {
        fn inner(tbl: &ResTblReader, needle: TableIndex) -> Option<TableEntry> {
            match needle {
                TableIndex::HashIndex(hash) => tbl.find_hash_entry(hash).map(TableEntry::Hash),
                TableIndex::StringIndex(name) => tbl
                    .find_name_entry(&name)
                    .map(TableEntry::Name)
                    .or_else(|| {
                        let hash = hash_name(&name);
                        tbl.find_hash_entry(hash).map(TableEntry::Hash)
                    }),
            }
        }
        inner(self, needle.into())
    }

    /// Iterate all RSTB entries across both the hash and name tables.
    pub fn iter(&self) -> ResTblIterator<'_> {
        ResTblIterator {
            table: self,
            index: 0,
        }
    }

    #[inline(always)]
    fn name_table_offset(&self) -> usize {
        Header::FULL_SIZE + self.header.crc_table_count as usize * size_of::<HashEntry>()
    }

    #[inline(always)]
    fn hash_table_index(&self, index: usize) -> Option<HashTableIndex> {
        (index < self.header.crc_table_count as usize).then_some(HashTableIndex(index))
    }
}

#[cfg(feature = "alloc")]
impl super::ResourceSizeTable {
    /// Parse an owned table from binary form
    pub fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        fn inner(data: &[u8]) -> Result<super::ResourceSizeTable> {
            let parser = ResTblReader::new(data)?;
            let mut crc_table = alloc::collections::BTreeMap::new();
            let mut name_table = alloc::collections::BTreeMap::new();
            for entry in parser.iter() {
                match entry {
                    TableEntry::Hash(entry) => crc_table.insert(entry.hash(), entry.value()),
                    TableEntry::Name(entry) => name_table.insert(entry.name(), entry.value()),
                };
            }
            Ok(super::ResourceSizeTable {
                crc_table,
                name_table,
            })
        }
        inner(data.as_ref())
    }

    /// Write the table in its binary format to bytes.
    pub fn to_binary(&self) -> alloc::vec::Vec<u8> {
        let size = Header::FULL_SIZE
            + size_of::<HashEntry>() * self.crc_table.len()
            + size_of::<NameEntry>() * self.name_table.len();
        let mut buffer = alloc::vec![0u8; size];
        Header {
            version: 1,
            string_block_size: size_of::<Name>() as u32,
            crc_table_count: self.crc_table.len() as u32,
            name_table_count: self.name_table.len() as u32,
        }
        .write(&mut buffer);
        let mut pos = Header::FULL_SIZE;
        for (hash, value) in &self.crc_table {
            HashEntry {
                hash: *hash,
                value: *value,
            }
            .write(&mut buffer[pos..]);
            pos += size_of::<HashEntry>();
        }
        for (name, value) in &self.name_table {
            NameEntry {
                name: *name,
                value: *value,
            }
            .write(&mut buffer[pos..]);
            pos += size_of::<NameEntry>();
        }
        buffer
    }
}

#[cfg(test)]
mod test {
    use crate::test::DATA;

    #[test]
    fn parse() {
        let parser = super::ResTblReader::new(DATA).unwrap();
        dbg!(parser.header());
        #[cfg(feature = "alloc")]
        {
            let table = crate::ResourceSizeTable::from_parser(&parser);
            assert_eq!(
                parser.header.crc_table_count as usize,
                table.crc_table.len(),
            );
            assert_eq!(
                parser.header.name_table_count as usize,
                table.name_table.len(),
            );
        }
    }

    #[test]
    fn lookup() {
        let parser = super::ResTblReader::new(DATA).unwrap();
        let entry = parser
            .get("Cooking/CookingTable.game__cooking__Table.bgyml")
            .unwrap();
        dbg!(entry);
        assert!(parser.get("Pack/Actor/Nonexistent.pack").is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn serialize() {
        let table = crate::ResourceSizeTable::from_binary(DATA).unwrap();
        let bytes = table.to_binary();
        let table2 = crate::ResourceSizeTable::from_binary(&bytes).unwrap();
        assert_eq!(table.crc_table.len(), table2.crc_table.len());
        for (entry1, entry2) in table.crc_table.iter().zip(table2.crc_table.iter()) {
            assert_eq!(entry1, entry2);
        }
        assert_eq!(table.name_table.len(), table2.name_table.len());
        for (entry1, entry2) in table.name_table.iter().zip(table2.name_table.iter()) {
            assert_eq!(entry1, entry2);
        }
        assert_eq!(DATA, bytes);
    }
}
