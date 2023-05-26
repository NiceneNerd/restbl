#[derive(Clone, Copy)]
pub struct Name {
    inner: [u8; 160],
}

impl core::ops::Deref for Name {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl core::fmt::Debug for Name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("String")
            .field("length", &self.as_str().len())
            .field("value", &self.as_str())
            .finish()
    }
}

impl core::fmt::Display for Name {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<&str> for &Name {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Eq for Name {}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd<&str> for Name {
    fn partial_cmp(&self, other: &&str) -> Option<core::cmp::Ordering> {
        self.as_str().partial_cmp(*other)
    }
}

impl PartialOrd<&str> for &Name {
    fn partial_cmp(&self, other: &&str) -> Option<core::cmp::Ordering> {
        self.as_str().partial_cmp(*other)
    }
}

impl Name {
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        unsafe {
            let zero_idx = self.inner.iter().position(|c| *c == 0).unwrap_unchecked();
            core::str::from_utf8_unchecked(&self.inner[..zero_idx])
        }
    }
}

impl AsRef<str> for Name {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&[u8]> for Name {
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
        Err(crate::Error::InsufficientData(
            value.len() - offset,
            "4 bytes for u32",
        ))
    } else {
        Ok(u32::from_le_bytes(unsafe {
            value[offset..offset + 4].try_into().unwrap_unchecked()
        }))
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
