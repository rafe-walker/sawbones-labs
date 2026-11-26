//! Bounded little-endian readers and writers. Nothing here indexes past a
//! slice: every read is checked and returns a typed error.

use crate::{DecodeError, EncodeError};

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) const fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes }
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.bytes.len() < n {
            return Err(DecodeError::PayloadTooShort);
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, DecodeError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    pub(crate) fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub(crate) fn i64(&mut self) -> Result<i64, DecodeError> {
        Ok(i64::from_le_bytes(self.array()?))
    }

    pub(crate) fn bool(&mut self) -> Result<bool, DecodeError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::BadBool),
        }
    }

    /// A slice with a 2-byte length in front, at most `max` bytes long.
    pub(crate) fn sized(
        &mut self,
        max: usize,
        field: &'static str,
    ) -> Result<&'a [u8], DecodeError> {
        let len = usize::from(self.u16()?);
        if len > max {
            return Err(DecodeError::OutOfBounds { field });
        }
        self.take(len)
    }

    pub(crate) fn finish(self) -> Result<(), DecodeError> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(DecodeError::PayloadTrailing)
        }
    }
}

pub(crate) struct Writer<'a> {
    out: &'a mut [u8],
    at: usize,
}

impl<'a> Writer<'a> {
    pub(crate) fn new(out: &'a mut [u8]) -> Self {
        Writer { out, at: 0 }
    }

    pub(crate) const fn len(&self) -> usize {
        self.at
    }

    pub(crate) fn put(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        let end = self.at.saturating_add(bytes.len());
        match self.out.get_mut(self.at..end) {
            Some(slot) => {
                slot.copy_from_slice(bytes);
                self.at = end;
                Ok(())
            }
            None => Err(EncodeError::BufferTooSmall { needed: end }),
        }
    }

    pub(crate) fn u8(&mut self, v: u8) -> Result<(), EncodeError> {
        self.put(&[v])
    }

    pub(crate) fn u16(&mut self, v: u16) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn u32(&mut self, v: u32) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn u64(&mut self, v: u64) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn i32(&mut self, v: i32) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn i64(&mut self, v: i64) -> Result<(), EncodeError> {
        self.put(&v.to_le_bytes())
    }

    pub(crate) fn bool(&mut self, v: bool) -> Result<(), EncodeError> {
        self.u8(u8::from(v))
    }

    /// A slice with a 2-byte length in front. The caller has checked the
    /// bound.
    pub(crate) fn sized(
        &mut self,
        bytes: &[u8],
        max: usize,
        field: &'static str,
    ) -> Result<(), EncodeError> {
        let len = match u16::try_from(bytes.len()) {
            Ok(len) if bytes.len() <= max => len,
            _ => return Err(EncodeError::Invalid(DecodeError::OutOfBounds { field })),
        };
        self.u16(len)?;
        self.put(bytes)
    }
}
