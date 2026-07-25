use std::fmt;

use destack_core::{SectionEntry, SectionImage, SectionSlice, SectionStorage};
use destack_serde::Reflect;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    CodeOffset, FrameMap, Function, FunctionId, Instruction, Instructions, Parameter, RegisterSpan,
    Relocation, Result,
};

/// Relocatable Destack bytecode for one module.
#[derive(Clone, Debug, Reflect)]
pub struct Object {
    /// Complete aligned object storage.
    storage: SectionStorage,
}

/// Bytecode object load failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectLoadError {
    /// The byte region cannot contain an object header.
    Truncated,
    /// The byte region does not satisfy object alignment.
    Misaligned,
    /// The byte region does not contain Destack bytecode.
    InvalidMagic,
    /// The header length does not match the byte region.
    InvalidLength,
    /// One typed section lies outside the byte region or violates entry alignment.
    InvalidSection,
}

impl fmt::Display for ObjectLoadError {
    /// Format one bytecode object load failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("truncated bytecode object"),
            Self::Misaligned => formatter.write_str("misaligned bytecode object"),
            Self::InvalidMagic => formatter.write_str("invalid bytecode object magic"),
            Self::InvalidLength => formatter.write_str("invalid bytecode object length"),
            Self::InvalidSection => formatter.write_str("invalid bytecode object section"),
        }
    }
}

impl std::error::Error for ObjectLoadError {}

/// Fixed header stored at byte zero of every bytecode object.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, SectionEntry)]
pub(super) struct Header {
    /// Stable object format marker.
    pub(super) magic: u32,
    /// Reserved header bytes.
    pub(super) reserved: u32,
    /// Complete object image byte length.
    pub(super) byte_len: u64,
    /// Physical functions in object-local function order.
    pub(super) functions: SectionSlice<Function>,
    /// Flattened physical function parameters.
    pub(super) parameters: SectionSlice<Parameter>,
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

    /// Create one empty bytecode object header.
    pub(super) fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            reserved: 0,
            byte_len: 0,
            functions: SectionSlice::empty(),
            parameters: SectionSlice::empty(),
            frames: SectionSlice::empty(),
            registers: SectionSlice::empty(),
            operations: SectionSlice::empty(),
            relocations: SectionSlice::empty(),
            code: SectionSlice::empty(),
        }
    }

    /// Load one bytecode object directly from aligned immutable bytes.
    fn load(bytes: &[u8]) -> std::result::Result<&Self, ObjectLoadError> {
        if bytes.len() < size_of::<Self>() {
            return Err(ObjectLoadError::Truncated);
        }
        if bytes.as_ptr().align_offset(align_of::<Self>()) != 0 {
            return Err(ObjectLoadError::Misaligned);
        }

        // SAFETY: the byte region is large and aligned enough for the fixed header.
        let header = unsafe { &*bytes.as_ptr().cast::<Self>() };
        if header.magic != Self::MAGIC {
            return Err(ObjectLoadError::InvalidMagic);
        }
        if usize::try_from(header.byte_len).ok() != Some(bytes.len()) {
            return Err(ObjectLoadError::InvalidLength);
        }

        // require every typed section to fit the mapped image
        header.check_section(header.functions)?;
        header.check_section(header.parameters)?;
        header.check_section(header.frames)?;
        header.check_section(header.registers)?;
        header.check_section(header.operations)?;
        header.check_section(header.relocations)?;
        header.check_section(header.code)?;

        Ok(header)
    }

    /// Require one typed section to fit this object image.
    fn check_section<T: SectionEntry>(
        &self,
        section: SectionSlice<T>,
    ) -> std::result::Result<(), ObjectLoadError> {
        let byte_offset = section.byte_offset as usize;
        let Some(byte_len) = section.len().checked_mul(size_of::<T>()) else {
            return Err(ObjectLoadError::InvalidSection);
        };
        let Some(byte_end) = byte_offset.checked_add(byte_len) else {
            return Err(ObjectLoadError::InvalidSection);
        };
        let is_aligned = byte_offset.is_multiple_of(align_of::<T>());
        if byte_end > self.byte_len as usize || !is_aligned {
            return Err(ObjectLoadError::InvalidSection);
        }

        Ok(())
    }
}

impl Object {
    /// Retain one compiler-built object image.
    pub(super) fn from_storage(storage: SectionStorage) -> Self {
        Self { storage }
    }

    /// Load one compiler-produced object from retained aligned storage.
    pub fn load(storage: SectionStorage) -> std::result::Result<Self, ObjectLoadError> {
        Header::load(storage.bytes())?;

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
        SectionImage::new(&self.storage)
    }

    /// Return physical functions in object-local function order.
    pub fn functions(&self) -> &[Function] {
        self.sections().entries(self.header().functions)
    }

    /// Return one object-local physical function.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.functions().get(function.index())
    }

    /// Return flattened physical function parameters.
    pub fn parameters(&self) -> &[Parameter] {
        self.sections().entries(self.header().parameters)
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
const _: () = assert!(size_of::<Header>() == 128);
