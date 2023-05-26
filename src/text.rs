use super::*;

impl bin::ResTblReader<'_> {
    fn write_text_to_buf_unchecked(&self, buffer: &mut [u8]) -> usize {
        let mut pos = 0;
        for entry in self.iter() {
            match entry {
                bin::TableEntry::Hash(entry) => {
                    pos += lexical_core::write(entry.hash(), &mut buffer[pos..]).len();
                    buffer[pos..pos + 2].copy_from_slice(b": ".as_slice());
                    pos += 2;
                    pos += lexical_core::write(entry.value(), &mut buffer[pos..]).len();
                    buffer[pos] = b'\n';
                    pos += 1;
                }
                bin::TableEntry::Name(entry) => {
                    let name = entry.name();
                    buffer[pos..pos + name.len()].copy_from_slice(name.as_bytes());
                    pos += name.len();
                    buffer[pos..pos + 2].copy_from_slice(b": ".as_slice());
                    pos += 2;
                    pos += lexical_core::write(entry.value(), &mut buffer[pos..]).len();
                    buffer[pos] = b'\n';
                    pos += 1;
                }
            }
        }
        pos
    }

    pub fn write_text_to_buf(&self, buffer: &mut [u8]) -> Result<usize> {
        let min_crc_size = self.header().crc_table_count() as usize
            * (<u32 as lexical_core::FormattedSize>::FORMATTED_SIZE * 2 + 3);
        let min_name_size = self.header().name_table_count() as usize
            * (160 + <u32 as lexical_core::FormattedSize>::FORMATTED_SIZE + 3);
        let min_size = min_crc_size + min_name_size;
        if buffer.len() < min_size {
            Err(Error::InsufficientBuffer(buffer.len(), min_size))
        } else {
            Ok(self.write_text_to_buf_unchecked(buffer))
        }
    }

    #[cfg(feature = "std")]
    pub fn write_text(&self, mut writer: impl std::io::Write) -> Result<()> {
        for entry in self.iter() {
            match entry {
                bin::TableEntry::Hash(entry) => {
                    writeln!(writer, "{}: {}", entry.hash(), entry.value())?;
                }
                bin::TableEntry::Name(entry) => {
                    writeln!(writer, "{}: {}", entry.name(), entry.value())?;
                }
            }
        }
        Ok(())
    }

    #[cfg(feature = "alloc")]
    pub fn to_text(&self) -> alloc::string::String {
        #[cfg(feature = "std")]
        {
            let min_crc_size = self.header().crc_table_count() as usize
                * (<u32 as lexical_core::FormattedSize>::FORMATTED_SIZE * 2 + 3);
            let min_name_size = self.header().name_table_count() as usize
                * (160 + <u32 as lexical_core::FormattedSize>::FORMATTED_SIZE + 3);
            let min_size = min_crc_size + min_name_size;
            let mut string = Vec::with_capacity(min_size);
            self.write_text(&mut string)
                .expect("Writing in-memory should never fail");
            unsafe { alloc::string::String::from_utf8_unchecked(string) }
        }
        #[cfg(not(feature = "std"))]
        {
            self.iter()
                .map(|entry| match entry {
                    bin::TableEntry::Hash(entry) => {
                        alloc::format!("{}: {}\n", entry.hash(), entry.value())
                    }
                    bin::TableEntry::Name(entry) => {
                        alloc::format!("{}: {}\n", entry.name(), entry.value())
                    }
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::test::DATA;
    #[test]
    #[cfg(feature = "alloc")]
    fn write_to_buf() {
        let mut buffer = vec![0u8; 1024 * 1024 * 10];
        let parser = crate::bin::ResTblReader::new(DATA).unwrap();
        let len = parser.write_text_to_buf(&mut buffer).unwrap();
        let text = core::str::from_utf8(&buffer[..len]).unwrap();
        println!("{text}");
    }

    #[test]
    #[cfg(feature = "std")]
    fn write_to_writer() {
        let mut buffer = Vec::with_capacity(1024 * 1024 * 10);
        let parser = crate::bin::ResTblReader::new(DATA).unwrap();
        parser.write_text(&mut buffer).unwrap();
        let text = String::from_utf8(buffer).unwrap();
        println!("{text}");
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn write_to_string() {
        let parser = crate::bin::ResTblReader::new(DATA).unwrap();
        let text = parser.to_text();
        println!("{text}");
    }
}
