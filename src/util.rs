pub struct String {
    inner: [u8; 160],
}

impl core::fmt::Debug for String {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("String")
            .field("length", &self.as_str().len())
            .field("value", &self.as_str())
            .finish()
    }
}

impl core::fmt::Display for String {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl PartialEq for String {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<&str> for String {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<&str> for &String {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Eq for String {}

impl PartialOrd for String {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Ord for String {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd<&str> for String {
    fn partial_cmp(&self, other: &&str) -> Option<core::cmp::Ordering> {
        self.as_str().partial_cmp(*other)
    }
}

impl PartialOrd<&str> for &String {
    fn partial_cmp(&self, other: &&str) -> Option<core::cmp::Ordering> {
        self.as_str().partial_cmp(*other)
    }
}

impl String {
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        unsafe {
            let zero_idx = self.inner.iter().position(|c| *c == 0).unwrap_unchecked();
            core::str::from_utf8_unchecked(&self.inner[..zero_idx])
        }
    }
}

impl AsRef<str> for String {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&[u8]> for String {
    type Error = crate::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut inner: [u8; 160] = unsafe { core::mem::zeroed() };
        let mut len = 0;
        for (dest, src) in inner.iter_mut().zip(value.iter()) {
            if *src != 0 {
                *dest = *src;
                len += 1;
            } else {
                break;
            }
        }
        Ok(core::str::from_utf8(&inner[..len]).map(|_| Self { inner })?)
    }
}

pub(crate) fn read_u32(value: &[u8], offset: Option<usize>) -> crate::Result<u32> {
    let offset = offset.unwrap_or_default();
    if value.len() < 4 + offset {
        Err(crate::Error::InsufficientData("4 bytes for u32"))
    } else {
        Ok(u32::from_le_bytes([
            value[offset],
            value[1 + offset],
            value[2 + offset],
            value[3 + offset],
        ]))
    }
}

/// CRC hash function matching that used in BOTW/TOTK.
#[inline]
pub const fn hash_name(name: &str) -> u32 {
    let mut crc = 0xFFFFFFFF;
    let mut i = 0;
    while i < name.len() {
        crc ^= name.as_bytes()[i] as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        i += 1;
    }
    !crc
}