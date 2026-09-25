use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tspp_core::{
    SectionEntry, SectionImage, SectionImageError, SectionLoader, SectionSlice, SectionStorage,
};
use tspp_serde::Reflect;

use crate::{
    CodeOffset, FrameMap, Function, FunctionId, Instruction, Instructions, RegisterSpan,
    Relocation, Result,
};

/// Relocatable TS++ bytecode for one module.
#[derive(Clone, Debug, Reflect)]
pub struct Object {
    /// Complete aligned object storage.
    storage: SectionStorage,
}

/// Bytecode object load failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectLoadError {
    /// The physical section image is malformed.
    Image(SectionImageError),
    /// The byte region does not contain TS++ bytecode.
    InvalidMagic,
    /// The bytecode object version is not supported.
    UnsupportedVersion(u16),
    /// The header length does not match the byte region.
    InvalidLength,
    /// One function or frame range is outside its containing section.
    InvalidRange,
}

impl fmt::Display for ObjectLoadError {
    /// Format one bytecode object load failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(error) => write!(formatter, "invalid bytecode object image: {error}"),
            Self::InvalidMagic => formatter.write_str("invalid bytecode object magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported bytecode object version {version}")
            }
            Self::InvalidLength => formatter.write_str("invalid bytecode object length"),
            Self::InvalidRange => formatter.write_str("invalid bytecode object range"),
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

/// Fixed header stored at byte zero of every bytecode object.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, SectionEntry)]
pub(super) struct Header {
    /// Stable object format marker.
    pub(super) magic: u32,
    /// Stable object format version.
    pub(super) version: u16,
    /// Reserved header word.
    pub(super) reserved: u16,
    /// Complete object image byte length.
    pub(super) byte_len: u64,
    /// Physical functions in object-local function order.
    pub(super) functions: SectionSlice<Function>,
    /// Physical frame maps in object-local frame state order.
    pub(super) frames: SectionSlice<FrameMap>,
    /// Flattened register spans referenced by frame maps.
    pub(super) registers: SectionSlice<RegisterSpan>,
    /// Function-relative byte offsets of logical operations.
    pub(super) operations: SectionSlice<CodeOffset>,
    /// Relocatable identity operands in function code.
    pub(super) relocations: SectionSlice<Relocation>,
    /// Contiguous instruction bytes for every defined function.
    pub(super) code: SectionSlice<u8>,
}

impl Header {
    /// The stable bytecode object marker.
    const MAGIC: u32 = u32::from_le_bytes(*b"DSBC");
    /// The stable bytecode object format version.
    const VERSION: u16 = 3;

    /// Create one empty bytecode object header.
    pub(super) fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            reserved: 0,
            byte_len: 0,
            functions: SectionSlice::empty(),
            frames: SectionSlice::empty(),
            registers: SectionSlice::empty(),
            operations: SectionSlice::empty(),
            relocations: SectionSlice::empty(),
            code: SectionSlice::empty(),
        }
    }

    /// Load one bytecode object directly from aligned immutable bytes.
    fn load(storage: &SectionStorage) -> std::result::Result<&Self, ObjectLoadError> {
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

        // SAFETY: every absolute section reachable from the header was validated above.
        let sections = unsafe { SectionImage::new(storage) };
        let functions = sections.entries(header.functions);
        let frames = sections.entries(header.frames);
        let registers = sections.entries(header.registers);
        let operations = sections.entries(header.operations);
        let code = sections.entries(header.code);

        // require every subordinate range used by infallible navigation
        for function in functions {
            if !function.operations.fits(operations.len()) {
                return Err(ObjectLoadError::InvalidRange);
            }
            if function.code().is_some_and(|range| !range.fits(code.len())) {
                return Err(ObjectLoadError::InvalidRange);
            }
        }
        for frame in frames {
            if !frame.registers.fits(registers.len()) {
                return Err(ObjectLoadError::InvalidRange);
            }
        }

        Ok(header)
    }
}

impl Object {
    /// Retain one compiler-built object image.
    pub(super) fn from_storage(storage: SectionStorage) -> Self {
        Self { storage }
    }

    /// Load one compiler-produced object from retained aligned storage.
    pub fn load(storage: SectionStorage) -> std::result::Result<Self, ObjectLoadError> {
        Header::load(&storage)?;

        Ok(Self::from_storage(storage))
    }

    /// Copy and load one bytecode object.
    pub fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, ObjectLoadError> {
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

    /// Return physical functions in object-local function order.
    pub fn functions(&self) -> &[Function] {
        self.sections().entries(self.header().functions)
    }

    /// Return one object-local physical function.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.functions().get(function.index())
    }

    /// Return physical frame maps in object-local frame state order.
    pub fn frames(&self) -> &[FrameMap] {
        self.sections().entries(self.header().frames)
    }

    /// Return flattened register spans referenced by frame maps.
    pub fn registers(&self) -> &[RegisterSpan] {
        self.sections().entries(self.header().registers)
    }

    /// Return function-relative byte offsets of logical operations.
    pub fn operations(&self) -> &[CodeOffset] {
        self.sections().entries(self.header().operations)
    }

    /// Return relocatable identity operands in function code.
    pub fn relocations(&self) -> &[Relocation] {
        self.sections().entries(self.header().relocations)
    }

    /// Return all encoded function bytes.
    pub fn code(&self) -> &[u8] {
        self.sections().entries(self.header().code)
    }

    /// Iterate over one defined function's instructions.
    pub fn instructions(&self, function: FunctionId) -> Option<Instructions<'_>> {
        let code = self.function(function)?.code()?;

        Some(code.instructions(self.code()))
    }

    /// Read one instruction by function-relative byte offset.
    pub fn instruction(
        &self,
        function: FunctionId,
        offset: CodeOffset,
    ) -> Result<Option<Instruction<'_>>> {
        let Some(function) = self.function(function) else {
            return Ok(None);
        };
        let Some(code) = function.code() else {
            return Ok(None);
        };

        code.instruction(self.code(), offset).map(Some)
    }

    /// Read one instruction by logical operation index.
    pub fn operation(
        &self,
        function: FunctionId,
        operation: u32,
    ) -> Result<Option<Instruction<'_>>> {
        let Some(function_row) = self.function(function) else {
            return Ok(None);
        };
        let Some(offset) = function_row.operation(self.operations(), operation) else {
            return Ok(None);
        };

        self.instruction(function, offset)
    }

    /// Return the fixed header at the start of this object image.
    fn header(&self) -> &Header {
        // SAFETY: Object constructors require a valid aligned header in retained storage.
        unsafe { &*self.storage.bytes().as_ptr().cast::<Header>() }
    }
}

impl Serialize for Object {
    /// Serialize this object as its complete aligned byte image.
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Object {
    /// Deserialize and check one complete object image.
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;

        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

const _: () = assert!(align_of::<Header>() == 16);
const _: () = assert!(size_of::<Header>() == 112);
