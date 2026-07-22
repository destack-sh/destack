use std::fmt;

use destack_core::{SectionEntry, SectionImage, SectionSlice, SectionStorage, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    CodeOffset, Constant, ConstantId, ConstantRelocation, DynamicRelocation, Error, FrameSlot,
    Function, FunctionId, Global, GlobalId, Instruction, InstructionRelocation, Instructions,
    StringEntry, TypeId, ValueType,
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
    /// The bytecode version is not supported.
    UnsupportedVersion(u16),
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
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported bytecode object version {version}")
            }
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
    /// Stable object format version.
    pub(super) version: u16,
    /// Reserved header word.
    pub(super) reserved: u16,
    /// Complete object image byte length.
    pub(super) byte_len: u64,

    /// Stable strings sorted by content id.
    pub(super) strings: SectionSlice<StringEntry>,
    /// Concatenated UTF-8 bytes referenced by string entries.
    pub(super) string_bytes: SectionSlice<u8>,

    /// Type symbols referenced by this object.
    pub(super) types: SectionSlice<StringId>,
    /// Flattened function value types.
    pub(super) value_types: SectionSlice<ValueType>,

    /// Global declarations and definitions.
    pub(super) globals: SectionSlice<Global>,
    /// Immutable object constants.
    pub(super) constants: SectionSlice<Constant>,
    /// Concatenated bytes referenced by constants.
    pub(super) constant_bytes: SectionSlice<u8>,

    /// Flattened frame slots referenced by functions.
    pub(super) frame_slots: SectionSlice<FrameSlot>,
    /// Function declarations and definitions.
    pub(super) functions: SectionSlice<Function>,
    /// Contiguous instruction bytes for every defined function.
    pub(super) code: SectionSlice<u8>,
    /// Relocatable operands in function instruction streams.
    pub(super) instruction_relocations: SectionSlice<InstructionRelocation>,
    /// Dynamic dispatch relocations in function instruction streams.
    pub(super) dynamic_relocations: SectionSlice<DynamicRelocation>,
    /// Relocatable operands in immutable constants.
    pub(super) constant_relocations: SectionSlice<ConstantRelocation>,
    /// Function-relative byte offsets for logical operations.
    pub(super) operation_offsets: SectionSlice<CodeOffset>,
}

impl Header {
    /// The stable bytecode object marker.
    const MAGIC: u32 = u32::from_le_bytes(*b"DSBC");
    /// The stable bytecode object version.
    const VERSION: u16 = 1;

    /// Create one empty bytecode object header.
    pub(super) fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            reserved: 0,
            byte_len: 0,
            strings: SectionSlice::empty(),
            string_bytes: SectionSlice::empty(),
            types: SectionSlice::empty(),
            value_types: SectionSlice::empty(),
            globals: SectionSlice::empty(),
            constants: SectionSlice::empty(),
            constant_bytes: SectionSlice::empty(),
            frame_slots: SectionSlice::empty(),
            functions: SectionSlice::empty(),
            code: SectionSlice::empty(),
            instruction_relocations: SectionSlice::empty(),
            dynamic_relocations: SectionSlice::empty(),
            constant_relocations: SectionSlice::empty(),
            operation_offsets: SectionSlice::empty(),
        }
    }

    /// Load one bytecode object directly from aligned immutable bytes.
    fn load(bytes: &[u8]) -> Result<&Self, ObjectLoadError> {
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
        if header.version != Self::VERSION {
            return Err(ObjectLoadError::UnsupportedVersion(header.version));
        }
        if usize::try_from(header.byte_len).ok() != Some(bytes.len()) {
            return Err(ObjectLoadError::InvalidLength);
        }

        // require every section descriptor to fit the mapped image
        header.check_sections()?;

        Ok(header)
    }

    /// Require every typed section to fit this object image.
    fn check_sections(&self) -> Result<(), ObjectLoadError> {
        self.check_section(self.strings)?;
        self.check_section(self.string_bytes)?;
        self.check_section(self.types)?;
        self.check_section(self.value_types)?;
        self.check_section(self.globals)?;
        self.check_section(self.constants)?;
        self.check_section(self.constant_bytes)?;
        self.check_section(self.frame_slots)?;
        self.check_section(self.functions)?;
        self.check_section(self.code)?;
        self.check_section(self.instruction_relocations)?;
        self.check_section(self.dynamic_relocations)?;
        self.check_section(self.constant_relocations)?;

        self.check_section(self.operation_offsets)
    }

    /// Require one typed section to fit this object image.
    fn check_section<T: SectionEntry>(
        &self,
        section: SectionSlice<T>,
    ) -> Result<(), ObjectLoadError> {
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
    pub fn load(storage: SectionStorage) -> Result<Self, ObjectLoadError> {
        Header::load(storage.bytes())?;

        Ok(Self::from_storage(storage))
    }

    /// Copy and load one bytecode object.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ObjectLoadError> {
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

    /// Return the fixed header at the start of this object image.
    fn header(&self) -> &Header {
        // SAFETY: Object constructors require a valid aligned header in retained storage.
        unsafe { &*self.storage.bytes().as_ptr().cast::<Header>() }
    }

    /// Return all stored strings.
    pub fn strings(&self) -> &[StringEntry] {
        self.sections().entries(self.header().strings)
    }

    /// Return all stored string bytes.
    pub fn string_bytes(&self) -> &[u8] {
        self.sections().entries(self.header().string_bytes)
    }

    /// Return one stored string by its stable id.
    pub fn string(&self, id: StringId) -> Option<&str> {
        let entries = self.strings();
        let index = entries.binary_search_by_key(&id, |entry| entry.id).ok()?;
        let entry = entries.get(index)?;
        let bytes = entry.bytes(self.string_bytes());

        std::str::from_utf8(bytes).ok()
    }

    /// Return all type symbols.
    pub fn types(&self) -> &[StringId] {
        self.sections().entries(self.header().types)
    }

    /// Return one type symbol name.
    pub fn type_name(&self, ty: TypeId) -> Option<StringId> {
        self.types().get(ty.index()).copied()
    }

    /// Return all flattened function value types.
    pub fn value_types(&self) -> &[ValueType] {
        self.sections().entries(self.header().value_types)
    }

    /// Return all global declarations and definitions.
    pub fn globals(&self) -> &[Global] {
        self.sections().entries(self.header().globals)
    }

    /// Return one global declaration or definition.
    pub fn global(&self, global: GlobalId) -> Option<&Global> {
        self.globals().get(global.index())
    }

    /// Return all immutable constants.
    pub fn constants(&self) -> &[Constant] {
        self.sections().entries(self.header().constants)
    }

    /// Return one immutable constant.
    pub fn constant(&self, constant: ConstantId) -> Option<&Constant> {
        self.constants().get(constant.index())
    }

    /// Return all immutable constant bytes.
    pub fn constant_bytes(&self) -> &[u8] {
        self.sections().entries(self.header().constant_bytes)
    }

    /// Return all frame slots.
    pub fn frame_slots(&self) -> &[FrameSlot] {
        self.sections().entries(self.header().frame_slots)
    }

    /// Return all function declarations and definitions.
    pub fn functions(&self) -> &[Function] {
        self.sections().entries(self.header().functions)
    }

    /// Return one function declaration or definition.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.functions().get(function.index())
    }

    /// Iterate over one defined function's instructions.
    pub fn instructions(&self, function: FunctionId) -> Option<Instructions<'_>> {
        let code = self.function(function)?.body.code()?;

        Some(code.instructions(self.code()))
    }

    /// Read one instruction by logical operation index.
    pub fn instruction(
        &self,
        function: FunctionId,
        operation: u32,
    ) -> Result<Option<Instruction<'_>>, Error> {
        let Some(function) = self.function(function) else {
            return Ok(None);
        };
        let Some(code) = function.body.code() else {
            return Ok(None);
        };
        let Some(offset) = function
            .body
            .operation_offset(self.operation_offsets(), operation)
        else {
            return Ok(None);
        };

        code.instruction(self.code(), offset).map(Some)
    }

    /// Return all encoded function bytes.
    pub fn code(&self) -> &[u8] {
        self.sections().entries(self.header().code)
    }

    /// Return all instruction relocations.
    pub fn instruction_relocations(&self) -> &[InstructionRelocation] {
        self.sections()
            .entries(self.header().instruction_relocations)
    }

    /// Return all dynamic dispatch relocations.
    pub fn dynamic_relocations(&self) -> &[DynamicRelocation] {
        self.sections().entries(self.header().dynamic_relocations)
    }

    /// Return all constant relocations.
    pub fn constant_relocations(&self) -> &[ConstantRelocation] {
        self.sections().entries(self.header().constant_relocations)
    }

    /// Return function-relative logical operation offsets.
    pub fn operation_offsets(&self) -> &[CodeOffset] {
        self.sections().entries(self.header().operation_offsets)
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

const _: () = assert!(align_of::<Header>() == 16);
const _: () = assert!(size_of::<Header>() == 240);
