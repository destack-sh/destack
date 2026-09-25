use std::ops::Range;

use serde::Serialize;
use tspp_core::{Blob, BlobId, BlobStore, StringId, StringPool};
use tspp_source::PackageId;

use crate::{ArtifactCacheError, BuildId};

/// Serde newtype name emitted by StringId's derived serializer.
const STRING_ID_NEWTYPE: &str = "StringId";

/// Mutable artifact pack byte writer.
pub(super) struct ArtifactPackEncoder {
    /// Encoded bytes.
    bytes: Vec<u8>,
}

impl ArtifactPackEncoder {
    /// Create one writer with space for its artifact records.
    pub(super) fn with_capacity(record_bytes: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(record_bytes),
        }
    }

    /// Append raw bytes.
    pub(super) fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// Return the next write offset.
    pub(super) fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Append one little-endian u32.
    pub(super) fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    /// Append one fixed-width value and collect its interned strings.
    pub(super) fn write_value<T: Serialize + ?Sized>(
        &mut self,
        value: &T,
        strings: &mut Vec<StringId>,
    ) -> Result<Range<usize>, ArtifactCacheError> {
        let mut visit = |name: &'static str, bytes: &[u8]| {
            if name == STRING_ID_NEWTYPE {
                strings.push(tspp_serde::from_slice_fixed(bytes)?);
            }

            Ok(())
        };
        let start = self.len();
        tspp_serde::append_fixed(value, &mut self.bytes, &mut visit)?;

        Ok(start..self.len())
    }

    /// Reserve one little-endian u32 and return its offset.
    pub(super) fn reserve_u32(&mut self) -> usize {
        let offset = self.len();
        self.write_u32(0);

        offset
    }

    /// Reserve one little-endian u64 and return its offset.
    pub(super) fn reserve_u64(&mut self) -> usize {
        let offset = self.len();
        self.write_u64(0);

        offset
    }

    /// Replace one reserved little-endian u64.
    pub(super) fn patch_u64(&mut self, offset: usize, value: u64) {
        self.bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    /// Replace one reserved bounded byte length.
    pub(super) fn patch_len(
        &mut self,
        offset: usize,
        len: usize,
    ) -> Result<(), ArtifactCacheError> {
        let len = u32::try_from(len).map_err(|_| {
            ArtifactCacheError::Invalid("artifact pack value exceeds 4 GiB".to_string())
        })?;
        self.patch_u32(offset, len);

        Ok(())
    }

    /// Append one bounded byte length.
    pub(super) fn write_len(&mut self, len: usize) -> Result<(), ArtifactCacheError> {
        let len = u32::try_from(len).map_err(|_| {
            ArtifactCacheError::Invalid("artifact pack value exceeds 4 GiB".to_string())
        })?;
        self.write_u32(len);

        Ok(())
    }

    /// Append one optional package identity.
    pub(super) fn write_package(&mut self, package: Option<PackageId>) {
        match package {
            Some(package) => {
                self.bytes.push(1);
                self.write_u64(package.raw());
            }
            None => self.bytes.push(0),
        }
    }

    /// Append one ordered interned string identity list.
    pub(super) fn write_string_ids(
        &mut self,
        strings: &[StringId],
    ) -> Result<(), ArtifactCacheError> {
        self.write_len(strings.len())?;
        for string in strings {
            self.write_u64(string.raw());
        }

        Ok(())
    }

    /// Append one ordered immutable Blob identity list.
    pub(super) fn write_blobs(&mut self, blobs: &[Blob]) -> Result<(), ArtifactCacheError> {
        self.write_len(blobs.len())?;
        for blob in blobs {
            self.write_bytes(&blob.id.bytes());
            self.write_u64(blob.byte_len);
        }

        Ok(())
    }

    /// Append one interned string.
    pub(super) fn write_string(
        &mut self,
        id: StringId,
        strings: &StringPool,
    ) -> Result<(), ArtifactCacheError> {
        self.write_u64(id.raw());
        let length = self.reserve_u32();
        let start = self.len();
        let text = strings.get_maybe(id).ok_or_else(|| {
            ArtifactCacheError::Invalid(format!(
                "missing interned string {id} while writing artifact pack"
            ))
        })?;
        self.bytes.extend_from_slice(text.as_bytes());
        let text_len = self.len() - start;
        self.patch_len(length, text_len)?;

        Ok(())
    }

    /// Append one immutable Blob.
    pub(super) fn write_blob(
        &mut self,
        blob: Blob,
        blobs: &BlobStore,
    ) -> Result<(), ArtifactCacheError> {
        self.write_bytes(&blob.id.bytes());
        self.write_u64(blob.byte_len);
        let length = self.reserve_u32();
        let start = self.len();
        let memory = blobs.open(blob).map_err(|error| {
            ArtifactCacheError::Invalid(format!(
                "failed to read Blob {blob} while writing artifact pack: {error}"
            ))
        })?;
        self.bytes.extend_from_slice(memory.bytes());
        let byte_len = self.len() - start;
        self.patch_len(length, byte_len)?;

        Ok(())
    }

    /// Finish the complete pack byte sequence.
    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }

    /// Append one little-endian u64.
    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    /// Replace one reserved little-endian u32.
    fn patch_u32(&mut self, offset: usize, value: u32) {
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}

/// Immutable artifact pack byte reader.
pub(super) struct ArtifactPackDecoder<'a> {
    /// Complete pack bytes.
    bytes: &'a [u8],
    /// Next unread byte offset.
    offset: usize,
}

impl<'a> ArtifactPackDecoder<'a> {
    /// Create one reader over complete pack bytes.
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    /// Read and validate the pack marker.
    pub(super) fn read_magic(&mut self, expected: &[u8]) -> Result<(), ArtifactCacheError> {
        let magic = self.read_bytes(expected.len())?;
        if magic != expected {
            return Err(ArtifactCacheError::Invalid(
                "artifact pack has an invalid marker".to_string(),
            ));
        }

        Ok(())
    }

    /// Read and validate the pack format.
    pub(super) fn read_format(&mut self, expected: u32) -> Result<(), ArtifactCacheError> {
        let format = self.read_u32()?;
        if format != expected {
            return Err(ArtifactCacheError::Invalid(format!(
                "pack format {format} is unsupported"
            )));
        }

        Ok(())
    }

    /// Read and validate the producing build.
    pub(super) fn read_build(&mut self, expected: BuildId) -> Result<(), ArtifactCacheError> {
        let build = self.read_bytes(expected.as_bytes().len())?;
        if build != expected.as_bytes() {
            return Err(ArtifactCacheError::Invalid(format!(
                "pack build does not match host build {expected}"
            )));
        }

        Ok(())
    }

    /// Read one optional package identity.
    pub(super) fn read_package(&mut self) -> Result<Option<PackageId>, ArtifactCacheError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(PackageId::new(self.read_u64()?))),
            tag => Err(ArtifactCacheError::Invalid(format!(
                "artifact pack has invalid package tag {tag}"
            ))),
        }
    }

    /// Return the next unread byte offset.
    pub(super) fn position(&self) -> usize {
        self.offset
    }

    /// Append one ordered interned string identity list.
    pub(super) fn read_string_ids(
        &mut self,
        strings: &mut Vec<StringId>,
    ) -> Result<Range<usize>, ArtifactCacheError> {
        let start = strings.len();
        let count = self.read_len()?;
        strings.reserve(count);
        for _ in 0..count {
            strings.push(StringId(self.read_u64()?));
        }
        let range = start..strings.len();
        if !strings[range.clone()].windows(2).all(|ids| ids[0] < ids[1]) {
            return Err(ArtifactCacheError::Invalid(
                "payload string identities are not strictly ordered".to_string(),
            ));
        }

        Ok(range)
    }

    /// Append one ordered immutable Blob identity list.
    pub(super) fn read_blob_ids(
        &mut self,
        blobs: &mut Vec<Blob>,
    ) -> Result<Range<usize>, ArtifactCacheError> {
        let start = blobs.len();
        let count = self.read_len()?;
        blobs.reserve(count);
        for _ in 0..count {
            let id = BlobId::new(self.read_array::<32>()?);
            let byte_len = self.read_u64()?;
            blobs.push(Blob::new(id, byte_len));
        }
        let range = start..blobs.len();
        if !blobs[range.clone()]
            .windows(2)
            .all(|items| items[0].id < items[1].id)
        {
            return Err(ArtifactCacheError::Invalid(
                "payload Blob identities are not strictly ordered".to_string(),
            ));
        }

        Ok(range)
    }

    /// Read and validate sorted interned strings.
    pub(super) fn read_strings(
        &mut self,
    ) -> Result<Vec<(StringId, Range<usize>)>, ArtifactCacheError> {
        let count = self.read_len()?;
        let mut strings = Vec::with_capacity(count);
        for _ in 0..count {
            let id = StringId(self.read_u64()?);
            let text = self.read_byte_range()?;
            let value = std::str::from_utf8(&self.bytes[text.clone()])
                .map_err(|_| ArtifactCacheError::Invalid("pack string is not UTF-8".to_string()))?;
            if StringId::for_text(value) != id {
                return Err(ArtifactCacheError::Invalid(format!(
                    "pack contains invalid string {id}"
                )));
            }
            strings.push((id, text));
        }

        Ok(strings)
    }

    /// Read and validate sorted immutable Blobs.
    pub(super) fn read_blobs(&mut self) -> Result<Vec<(Blob, Range<usize>)>, ArtifactCacheError> {
        let count = self.read_len()?;
        let mut blobs = Vec::with_capacity(count);
        for _ in 0..count {
            let id = self.read_array::<32>()?;
            let byte_len = self.read_u64()?;
            let bytes = self.read_byte_range()?;
            let blob = Blob::new(BlobId::new(id), byte_len);
            let actual = Blob::for_bytes(&self.bytes[bytes.clone()]);
            if actual != blob {
                return Err(ArtifactCacheError::Invalid(format!(
                    "Blob {blob} does not match packed bytes {actual}"
                )));
            }
            blobs.push((blob, bytes));
        }

        Ok(blobs)
    }

    /// Read one bounded byte offset.
    pub(super) fn read_offset(&mut self) -> Result<usize, ArtifactCacheError> {
        usize::try_from(self.read_u64()?).map_err(|_| {
            ArtifactCacheError::Invalid("artifact pack offset exceeds usize".to_string())
        })
    }

    /// Read one bounded byte length.
    pub(super) fn read_len(&mut self) -> Result<usize, ArtifactCacheError> {
        Ok(self.read_u32()? as usize)
    }

    /// Read one length-prefixed byte sequence.
    pub(super) fn read_byte_slice(&mut self) -> Result<&'a [u8], ArtifactCacheError> {
        let len = self.read_len()?;

        self.read_bytes(len)
    }

    /// Move to one exact validated byte offset.
    pub(super) fn seek(&mut self, offset: usize) -> Result<(), ArtifactCacheError> {
        if offset > self.bytes.len() {
            return Err(ArtifactCacheError::Invalid(
                "artifact pack offset lies outside the file".to_string(),
            ));
        }
        self.offset = offset;

        Ok(())
    }

    /// Require complete input consumption.
    pub(super) fn finish(&self) -> Result<(), ArtifactCacheError> {
        if self.offset != self.bytes.len() {
            return Err(ArtifactCacheError::Invalid(
                "artifact pack has trailing bytes".to_string(),
            ));
        }

        Ok(())
    }

    /// Read one byte.
    fn read_u8(&mut self) -> Result<u8, ArtifactCacheError> {
        let bytes = self.read_bytes(1)?;

        Ok(bytes[0])
    }

    /// Read one little-endian u32.
    fn read_u32(&mut self) -> Result<u32, ArtifactCacheError> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    /// Read one little-endian u64.
    fn read_u64(&mut self) -> Result<u64, ArtifactCacheError> {
        Ok(u64::from_le_bytes(self.read_array()?))
    }

    /// Read one length-prefixed byte range.
    fn read_byte_range(&mut self) -> Result<Range<usize>, ArtifactCacheError> {
        let len = self.read_len()?;
        let start = self.offset;
        self.read_bytes(len)?;

        Ok(start..self.offset)
    }

    /// Read one fixed byte array.
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ArtifactCacheError> {
        let bytes = self.read_bytes(N)?;
        let mut array = [0; N];
        array.copy_from_slice(bytes);

        Ok(array)
    }

    /// Read one exact byte sequence.
    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ArtifactCacheError> {
        let end = self.offset.checked_add(len).ok_or_else(|| {
            ArtifactCacheError::Invalid("artifact pack byte range overflow".to_string())
        })?;
        if end > self.bytes.len() {
            return Err(ArtifactCacheError::Invalid(
                "artifact pack ends inside a value".to_string(),
            ));
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;

        Ok(bytes)
    }
}
