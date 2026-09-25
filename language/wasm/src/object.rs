use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tspp_core::{
    SectionBuilder, SectionEntry, SectionImage, SectionImageError, SectionLoader, SectionSlice,
    SectionStorage,
};
use tspp_serde::Reflect;

/// Relocatable WebAssembly object.
#[derive(Clone, Debug, Reflect)]
pub struct Object {
    /// Complete aligned object storage.
    storage: SectionStorage,
}

/// WebAssembly object load failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectLoadError {
    /// The physical section image is malformed.
    Image(SectionImageError),
    /// The byte region does not contain a TS++ WebAssembly object.
    InvalidMagic,
    /// The WebAssembly object version is not supported.
    UnsupportedVersion(u16),
    /// The header length does not match the byte region.
    InvalidLength,
}

impl fmt::Display for ObjectLoadError {
    /// Format one WebAssembly object load failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(error) => write!(formatter, "invalid WebAssembly object image: {error}"),
            Self::InvalidMagic => formatter.write_str("invalid WebAssembly object magic"),
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "unsupported WebAssembly object version {version}"
                )
            }
            Self::InvalidLength => formatter.write_str("invalid WebAssembly object length"),
        }
    }
}

impl std::error::Error for ObjectLoadError {}

impl From<SectionImageError> for ObjectLoadError {
    /// Convert one malformed physical section image.
    fn from(error: SectionImageError) -> Self {
        Self::Image(error)
    }
}

/// Fixed header stored at byte zero of every WebAssembly object.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, SectionEntry)]
struct ObjectHeader {
    /// Stable object format marker.
    magic: u32,
    /// Stable object format version.
    version: u16,
    /// Reserved header word.
    reserved: u16,
    /// Complete object image byte length.
    byte_len: u64,
    /// Encoded relocatable WebAssembly module.
    module: SectionSlice<u8>,
}

impl ObjectHeader {
    /// The stable WebAssembly object marker.
    const MAGIC: u32 = u32::from_le_bytes(*b"DSWO");
    /// The stable WebAssembly object format version.
    const VERSION: u16 = 1;

    /// Create one empty WebAssembly object header.
    const fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            reserved: 0,
            byte_len: 0,
            module: SectionSlice::empty(),
        }
    }

    /// Load one WebAssembly object header and its absolute sections.
    fn load(storage: &SectionStorage) -> Result<&Self, ObjectLoadError> {
        let loader = SectionLoader::new(storage)?;
        let header = loader.header::<Self>()?;
        if header.magic != Self::MAGIC {
            return Err(ObjectLoadError::InvalidMagic);
        }
        if header.version != Self::VERSION {
            return Err(ObjectLoadError::UnsupportedVersion(header.version));
        }
        if usize::try_from(header.byte_len).ok() != Some(loader.bytes().len()) {
            return Err(ObjectLoadError::InvalidLength);
        }

        Ok(header)
    }
}

impl Object {
    /// Build one object around an encoded relocatable WebAssembly module.
    pub fn new(module: impl AsRef<[u8]>) -> Self {
        let mut sections = SectionBuilder::new();
        let mut header = ObjectHeader::new();
        let header_section = sections.insert([header]);

        // retain the encoded module in the object image
        header.module = sections.insert(module);

        // finalize the fixed header after all section offsets are known
        header.byte_len = sections.view().byte_len() as u64;
        sections.replace(header_section, [header]);

        Self {
            storage: sections.build(),
        }
    }

    /// Load one WebAssembly object from retained aligned storage.
    pub fn load(storage: SectionStorage) -> Result<Self, ObjectLoadError> {
        ObjectHeader::load(&storage)?;

        Ok(Self { storage })
    }

    /// Copy and load one complete WebAssembly object image.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ObjectLoadError> {
        Self::load(SectionStorage::from_bytes(bytes))
    }

    /// Return the encoded relocatable WebAssembly module.
    pub fn module(&self) -> &[u8] {
        self.sections().entries(self.header().module)
    }

    /// Return the complete mapped object bytes.
    pub fn bytes(&self) -> &[u8] {
        self.storage.bytes()
    }

    /// Return a read-only image of this object's sections.
    fn sections(&self) -> SectionImage<'_> {
        // SAFETY: Object construction validates every absolute section reachable from its header.
        unsafe { SectionImage::new(&self.storage) }
    }

    /// Return the fixed header at the start of this object image.
    fn header(&self) -> &ObjectHeader {
        // SAFETY: Object constructors require a valid aligned header in retained storage.
        unsafe { &*self.storage.bytes().as_ptr().cast::<ObjectHeader>() }
    }
}

impl Serialize for Object {
    /// Serialize this object as its complete aligned byte image.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Object {
    /// Deserialize and load one complete object image.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;

        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

const _: () = assert!(align_of::<ObjectHeader>() == 16);
const _: () = assert!(size_of::<ObjectHeader>() == 32);

#[cfg(test)]
mod tests {
    use std::mem::{offset_of, size_of};

    use tspp_core::SectionImageError;

    use super::{Object, ObjectHeader, ObjectLoadError};

    /// Load one WebAssembly module from its complete immutable object image.
    #[test]
    fn test_load_object_image() {
        let object = Object::new(b"\0asm\x01\0\0\0");
        let loaded = Object::from_bytes(object.bytes()).expect("load WebAssembly object");

        // preserve the complete object and its embedded module
        assert_eq!(loaded.bytes(), object.bytes());
        assert_eq!(loaded.module(), b"\0asm\x01\0\0\0");

        // reject an absolute module section outside the image
        let offset = offset_of!(ObjectHeader, module);
        let mut invalid = object.bytes().to_vec();
        invalid[offset..offset + size_of::<u64>()].copy_from_slice(&u64::MAX.to_ne_bytes());
        let error = Object::from_bytes(&invalid).expect_err("reject invalid module section");
        assert_eq!(
            error,
            ObjectLoadError::Image(SectionImageError::InvalidSection)
        );
    }
}
