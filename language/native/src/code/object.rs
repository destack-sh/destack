use std::fmt;

use destack_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage,
    SectionImageError, SectionLoader, SectionSlice, SectionStorage,
};
use destack_serde::Reflect;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::abi;

use super::{CodeMap, CodeMapBuilder};

/// Relocatable native object for one module.
#[derive(Clone, Debug, Reflect)]
pub struct Object {
    /// Complete aligned object storage.
    storage: SectionStorage,
}

/// Native object load failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectLoadError {
    /// The physical section image is malformed.
    Image(SectionImageError),
    /// The byte region does not contain a Destack native object.
    InvalidMagic,
    /// The header length does not match the byte region.
    InvalidLength,
    /// One native object string is not valid UTF-8.
    InvalidString,
    /// One relative object range escapes its owning column.
    InvalidRange,
}

/// Fixed header stored at byte zero of every native object.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, SectionEntry)]
struct ObjectHeader {
    /// Stable native object format marker.
    magic: u32,
    /// Native object file format.
    format: ObjectFormat,
    /// Complete object image byte length.
    byte_len: u64,
    /// Target triple bytes inside the string column.
    target: EntryRange<u8>,
    /// Native module symbol bytes inside the string column.
    module: EntryRange<u8>,
    /// Exported symbols in object-local function order.
    entries: SectionSlice<Optional<EntryRange<u8>>>,
    /// Contiguous object-local string bytes.
    strings: SectionSlice<u8>,
    /// Runtime operations imported by this object.
    imports: SectionSlice<abi::Operation>,
    /// Physical frame maps inside this object.
    map: CodeMap,
    /// Relocatable platform object bytes.
    image: SectionSlice<u8>,
}

impl fmt::Display for ObjectLoadError {
    /// Format one native object load failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(error) => write!(formatter, "invalid native object image: {error}"),
            Self::InvalidMagic => formatter.write_str("invalid native object magic"),
            Self::InvalidLength => formatter.write_str("invalid native object length"),
            Self::InvalidString => formatter.write_str("invalid native object string"),
            Self::InvalidRange => formatter.write_str("invalid native object range"),
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

impl ObjectHeader {
    /// Stable native object marker.
    const MAGIC: u32 = u32::from_le_bytes(*b"DSNO");

    /// Create one empty native object header.
    fn new(format: ObjectFormat) -> Self {
        Self {
            magic: Self::MAGIC,
            format,
            byte_len: 0,
            target: EntryRange::empty(),
            module: EntryRange::empty(),
            entries: SectionSlice::empty(),
            strings: SectionSlice::empty(),
            imports: SectionSlice::empty(),
            map: CodeMap::empty(),
            image: SectionSlice::empty(),
        }
    }

    /// Load one header directly from aligned immutable bytes.
    fn load(storage: &SectionStorage) -> Result<&Self, ObjectLoadError> {
        let loader = SectionLoader::new(storage)?;
        let header = loader.header::<Self>()?;
        if header.magic != Self::MAGIC {
            return Err(ObjectLoadError::InvalidMagic);
        }
        if usize::try_from(header.byte_len).ok() != Some(loader.bytes().len()) {
            return Err(ObjectLoadError::InvalidLength);
        }
        // SAFETY: every absolute section reachable from the header was validated above.
        let sections = unsafe { SectionImage::new(storage) };
        let entries = sections.entries(header.entries);
        let strings = sections.entries(header.strings);

        // require every subordinate range used by infallible navigation
        header.check_strings(entries, strings)?;
        if !header.map.ranges_fit(sections) {
            return Err(ObjectLoadError::InvalidRange);
        }

        Ok(header)
    }

    /// Require every nested string section to contain valid UTF-8.
    fn check_strings(
        &self,
        entries: &[Optional<EntryRange<u8>>],
        strings: &[u8],
    ) -> Result<(), ObjectLoadError> {
        if !self.target.fits(strings.len()) || !self.module.fits(strings.len()) {
            return Err(ObjectLoadError::InvalidString);
        }
        Self::check_string(self.target.slice(strings))?;
        Self::check_string(self.module.slice(strings))?;

        // validate every optional exported symbol
        for entry in entries {
            let Some(entry) = entry.get() else {
                continue;
            };
            if !entry.fits(strings.len()) {
                return Err(ObjectLoadError::InvalidString);
            }
            Self::check_string(entry.slice(strings))?;
        }

        Ok(())
    }

    /// Require one byte slice to contain valid UTF-8.
    fn check_string(bytes: &[u8]) -> Result<(), ObjectLoadError> {
        std::str::from_utf8(bytes)
            .map(|_| ())
            .map_err(|_| ObjectLoadError::InvalidString)
    }
}

impl Object {
    /// Retain one compiler-built object image.
    fn from_storage(storage: SectionStorage) -> Self {
        Self { storage }
    }

    /// Load one compiler-produced object from retained aligned storage.
    pub fn load(storage: SectionStorage) -> Result<Self, ObjectLoadError> {
        ObjectHeader::load(&storage)?;

        Ok(Self::from_storage(storage))
    }

    /// Copy and load one native object.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ObjectLoadError> {
        Self::load(SectionStorage::from_bytes(bytes))
    }

    /// Return the complete mapped object bytes.
    pub fn bytes(&self) -> &[u8] {
        self.storage.bytes()
    }

    /// Return a read-only image of this object's sections.
    pub fn sections(&self) -> SectionImage<'_> {
        // SAFETY: Object construction validates every directly accessible typed section.
        unsafe { SectionImage::new(&self.storage) }
    }

    /// Return the target triple.
    pub fn target(&self) -> &str {
        self.string(self.header().target)
    }

    /// Return the native object format.
    pub fn format(&self) -> ObjectFormat {
        self.header().format
    }

    /// Return the relocatable platform object bytes.
    pub fn image(&self) -> &[u8] {
        self.sections().entries(self.header().image)
    }

    /// Return the native module symbol.
    pub fn module(&self) -> &str {
        self.string(self.header().module)
    }

    /// Iterate exported symbols in object-local function order.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = Option<&str>> {
        let sections = self.sections();

        sections
            .entries(self.header().entries)
            .iter()
            .map(move |entry| entry.get().map(|entry| self.string(entry)))
    }

    /// Return imported runtime operations.
    pub fn imports(&self) -> &[abi::Operation] {
        self.sections().entries(self.header().imports)
    }

    /// Return physical native frame maps.
    pub fn map(&self) -> CodeMap {
        self.header().map
    }

    /// Return the fixed header at the start of this object image.
    fn header(&self) -> &ObjectHeader {
        // SAFETY: Object constructors require a valid aligned header in retained storage.
        unsafe { &*self.storage.bytes().as_ptr().cast::<ObjectHeader>() }
    }

    /// Return one validated string section.
    fn string(&self, range: EntryRange<u8>) -> &str {
        let strings = self.sections().entries(self.header().strings);
        let bytes = range.slice(strings);

        // SAFETY: Object constructors validate every string section.
        unsafe { std::str::from_utf8_unchecked(bytes) }
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
    /// Deserialize and check one complete object image.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;

        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

/// Relocatable native object under construction.
#[derive(Debug)]
pub struct ObjectBuilder {
    /// The target triple.
    target: String,
    /// The object file format.
    format: ObjectFormat,
    /// Relocatable platform object bytes.
    image: Vec<u8>,
    /// Native module symbol imported by this object.
    module: String,
    /// Exported symbols in object-local function order.
    entries: Vec<Option<String>>,
    /// Runtime operations imported by this object.
    imports: Vec<abi::Operation>,
    /// Physical native frame maps.
    map: CodeMapBuilder,
}

impl ObjectBuilder {
    /// Create one relocatable native object builder.
    pub fn new(target: String, format: ObjectFormat, image: Vec<u8>, module: String) -> Self {
        Self {
            target,
            format,
            image,
            module,
            entries: Vec::new(),
            imports: Vec::new(),
            map: CodeMapBuilder::new(),
        }
    }

    /// Set exported symbols in object-local function order.
    pub fn entries(mut self, entries: impl IntoIterator<Item = Option<String>>) -> Self {
        self.entries = entries.into_iter().collect();

        self
    }

    /// Set imported runtime operations.
    pub fn imports(mut self, imports: impl IntoIterator<Item = abi::Operation>) -> Self {
        self.imports = imports.into_iter().collect();

        self
    }

    /// Set physical native frame maps.
    pub fn map(mut self, map: CodeMapBuilder) -> Self {
        self.map = map;

        self
    }

    /// Build one immutable native object.
    pub fn build(self) -> Object {
        let mut sections = SectionBuilder::new();
        let mut header = ObjectHeader::new(self.format);
        let header_section = sections.insert([header]);

        // pack object identity strings
        let mut strings = EntryStore::new();
        header.target = strings.append(self.target.bytes());
        header.module = strings.append(self.module.bytes());
        let entries = self
            .entries
            .into_iter()
            .map(|entry| entry.map(|entry| strings.append(entry.bytes())).into())
            .collect::<Vec<_>>();
        header.entries = sections.insert(entries);
        header.strings = sections.insert(strings.into_entries());

        // pack imports and physical frame maps
        header.imports = sections.insert(self.imports);
        header.map = self.map.build(&mut sections);

        // retain platform object bytes at the image alignment
        header.image = sections.insert_bytes(self.image, align_of::<u128>());

        // finalize the fixed header after all section offsets are known
        header.byte_len = sections.view().byte_len() as u64;
        sections.replace(header_section, [header]);

        Object::from_storage(sections.build())
    }
}

/// Native object file format.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum ObjectFormat {
    /// Executable and Linkable Format object.
    Elf = 0,
    /// Mach object file.
    MachO = 1,
    /// Common Object File Format object.
    Coff = 2,
}

const _: () = assert!(align_of::<ObjectHeader>() == 16);
const _: () = assert!(size_of::<ObjectHeader>() == 160);

#[cfg(test)]
mod tests {
    use destack_core::SectionImageError;

    use super::*;

    /// Load complete native objects and reject invalid nested entry representations.
    #[test]
    fn test_load_native_object_image() {
        let object = ObjectBuilder::new(
            "x86_64-unknown-linux-gnu".to_string(),
            ObjectFormat::Elf,
            vec![1, 2, 3, 4],
            "test.module".to_string(),
        )
        .entries([Some("test.entry".to_string())])
        .imports([abi::Operation::Poll])
        .build();
        let loaded = Object::from_bytes(object.bytes()).expect("native object should load");

        assert_eq!(loaded.target(), "x86_64-unknown-linux-gnu");
        assert_eq!(loaded.module(), "test.module");
        assert_eq!(loaded.entries().collect::<Vec<_>>(), [Some("test.entry")]);
        assert_eq!(loaded.imports(), [abi::Operation::Poll]);
        assert_eq!(loaded.image(), [1, 2, 3, 4]);

        // reject an unknown operation before constructing its enum value
        let mut bytes = object.bytes().to_vec();
        let operation = object.header().imports.byte_offset as usize;
        bytes[operation..operation + size_of::<u16>()].copy_from_slice(&u16::MAX.to_ne_bytes());
        let error = Object::from_bytes(&bytes)
            .expect_err("invalid native operation discriminant must be rejected");

        assert_eq!(
            error,
            ObjectLoadError::Image(SectionImageError::InvalidEntry)
        );
    }
}
