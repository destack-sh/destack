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

/// One defined function inside a relocatable native object.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// Internal native function body symbol bytes.
    body: EntryRange<u8>,
    /// Native runtime entry symbol bytes.
    entry: EntryRange<u8>,
    /// Compiled body byte length.
    pub body_byte_len: u32,
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
    /// Optional definitions in object-local function order.
    functions: SectionSlice<Optional<Function>>,
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
            functions: SectionSlice::empty(),
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
        let functions = sections.entries(header.functions);
        let strings = sections.entries(header.strings);

        // require every string used by infallible navigation
        if !header.strings_fit(functions, strings) {
            return Err(ObjectLoadError::InvalidString);
        }
        if !header.map.ranges_fit(sections) {
            return Err(ObjectLoadError::InvalidRange);
        }

        Ok(header)
    }

    /// Return whether every nested string is in bounds and valid UTF-8.
    fn strings_fit(&self, functions: &[Optional<Function>], strings: &[u8]) -> bool {
        let header_strings = [self.target, self.module];
        if !header_strings
            .into_iter()
            .all(|range| Self::string_fits(range, strings))
        {
            return false;
        }

        // validate every defined function symbol
        for function in functions {
            let Some(function) = function.get() else {
                continue;
            };
            let symbols = [function.body, function.entry];
            if !symbols
                .into_iter()
                .all(|range| Self::string_fits(range, strings))
            {
                return false;
            }
        }

        true
    }

    /// Return whether one string range is in bounds and valid UTF-8.
    fn string_fits(range: EntryRange<u8>, strings: &[u8]) -> bool {
        range.fits(strings.len()) && std::str::from_utf8(range.slice(strings)).is_ok()
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

    /// Return optional definitions in object-local function order.
    pub fn functions(&self) -> &[Optional<Function>] {
        self.sections().entries(self.header().functions)
    }

    /// Return one defined function's internal body symbol.
    pub fn body(&self, function: Function) -> &str {
        self.string(function.body)
    }

    /// Return one defined function's runtime entry symbol.
    pub fn entry(&self, function: Function) -> &str {
        self.string(function.entry)
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
    /// Optional definitions in object-local function order.
    functions: Vec<Option<FunctionBuilder>>,
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
            functions: Vec::new(),
            imports: Vec::new(),
            map: CodeMapBuilder::new(),
        }
    }

    /// Set optional definitions in object-local function order.
    pub fn functions(
        mut self,
        functions: impl IntoIterator<Item = Option<FunctionBuilder>>,
    ) -> Self {
        self.functions = functions.into_iter().collect();

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
        let functions = self
            .functions
            .into_iter()
            .map(|function| function.map(|function| function.build(&mut strings)).into())
            .collect::<Vec<_>>();
        header.functions = sections.insert(functions);
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

/// One relocatable native function under construction.
#[derive(Debug)]
pub struct FunctionBuilder {
    /// Internal native function body symbol.
    body: String,
    /// Native runtime entry symbol.
    entry: String,
    /// Compiled body byte length.
    body_byte_len: u32,
}

impl FunctionBuilder {
    /// Create one relocatable native function builder.
    pub fn new(body: String, entry: String, body_byte_len: u32) -> Self {
        Self {
            body,
            entry,
            body_byte_len,
        }
    }

    /// Pack this function into the object string column.
    fn build(self, strings: &mut EntryStore<u8>) -> Function {
        Function {
            body: strings.append(self.body.bytes()),
            entry: strings.append(self.entry.bytes()),
            body_byte_len: self.body_byte_len,
        }
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
