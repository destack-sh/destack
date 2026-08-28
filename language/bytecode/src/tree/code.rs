use destack_core::{SectionBuilder, SectionEntry, SectionImage, SectionImageError, SectionSlice};
use destack_serde::Reflect;
use destack_source::ProvenanceId;
use serde::{Deserialize, Serialize};

use crate::{Error, FrameMap, Function, Instruction, Instructions, RegisterSpan};

/// Linked executable bytecode stored in Program sections.
#[repr(C, align(8))]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct Code {
    /// Physical functions in Program function order.
    functions: SectionSlice<Function>,
    /// Physical frame maps in canonical frame state order.
    frames: SectionSlice<FrameMap>,
    /// Flattened register spans referenced by frame maps.
    registers: SectionSlice<RegisterSpan>,
    /// Function-relative byte offsets of logical operations.
    operations: SectionSlice<CodeOffset>,
    /// Provenance parallel to logical operation offsets.
    operation_provenances: SectionSlice<ProvenanceId>,
    /// Explicit physical mappings.
    mappings: SectionSlice<Mapping>,
    /// Contiguous linked instruction bytes.
    code: SectionSlice<u8>,
}

impl Code {
    /// Validate every linked bytecode range and parallel column.
    pub fn validate(&self, sections: SectionImage<'_>) -> Result<(), SectionImageError> {
        let functions = self.functions(sections);
        let frames = self.frames(sections);
        let operations = self.operations(sections);
        let operation_provenances = self.operation_provenances(sections);
        let mappings = self.mappings(sections);
        let bytes = self.bytes(sections);
        let registers = self.registers(sections);

        // validate parallel provenance columns
        if operations.len() != operation_provenances.len() {
            return Err(SectionImageError::InvalidRange);
        }

        // validate each function's flattened ranges
        for function in functions {
            function.validate(operations, mappings, bytes.len())?;
        }

        // validate each frame's retained register range
        for frame in frames {
            frame.validate(registers.len())?;
        }

        Ok(())
    }

    /// Return all linked bytecode functions.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [Function] {
        sections.entries(self.functions)
    }

    /// Return one linked function by its Program function index.
    pub fn function<'a>(
        &self,
        sections: SectionImage<'a>,
        function_index: usize,
    ) -> Option<&'a Function> {
        self.functions(sections).get(function_index)
    }

    /// Return physical frame maps in canonical frame state order.
    pub fn frames<'a>(&self, sections: SectionImage<'a>) -> &'a [FrameMap] {
        sections.entries(self.frames)
    }

    /// Return one physical frame map by canonical frame state index.
    pub fn frame<'a>(
        &self,
        sections: SectionImage<'a>,
        frame_index: usize,
    ) -> Option<&'a FrameMap> {
        self.frames(sections).get(frame_index)
    }

    /// Return flattened register spans referenced by frame maps.
    pub fn registers<'a>(&self, sections: SectionImage<'a>) -> &'a [RegisterSpan] {
        sections.entries(self.registers)
    }

    /// Return function-relative byte offsets of logical operations.
    pub fn operations<'a>(&self, sections: SectionImage<'a>) -> &'a [CodeOffset] {
        sections.entries(self.operations)
    }

    /// Return provenance parallel to logical operation offsets.
    pub fn operation_provenances<'a>(&self, sections: SectionImage<'a>) -> &'a [ProvenanceId] {
        sections.entries(self.operation_provenances)
    }

    /// Return explicit physical mappings.
    pub fn mappings<'a>(&self, sections: SectionImage<'a>) -> &'a [Mapping] {
        sections.entries(self.mappings)
    }

    /// Return contiguous linked instruction bytes.
    pub fn bytes<'a>(&self, sections: SectionImage<'a>) -> &'a [u8] {
        sections.entries(self.code)
    }

    /// Iterate one linked function's instructions.
    pub fn instructions<'a>(
        &self,
        sections: SectionImage<'a>,
        function_index: usize,
    ) -> Option<Instructions<'a>> {
        let function = self.function(sections, function_index)?;
        let code = function.code()?;

        Some(code.instructions(self.bytes(sections)))
    }

    /// Read one instruction by function-relative byte offset.
    pub fn instruction<'a>(
        &self,
        sections: SectionImage<'a>,
        function_index: usize,
        offset: CodeOffset,
    ) -> Result<Option<Instruction<'a>>, Error> {
        let Some(function) = self.function(sections, function_index) else {
            return Ok(None);
        };
        let Some(code) = function.code() else {
            return Ok(None);
        };

        code.instruction(self.bytes(sections), offset).map(Some)
    }

    /// Read one instruction by logical operation index.
    pub fn operation<'a>(
        &self,
        sections: SectionImage<'a>,
        function_index: usize,
        operation: u32,
    ) -> Result<Option<Instruction<'a>>, Error> {
        let Some(function) = self.function(sections, function_index) else {
            return Ok(None);
        };
        let Some(code) = function.code() else {
            return Ok(None);
        };
        let Some(offset) = function.operation(self.operations(sections), operation) else {
            return Ok(None);
        };

        code.instruction(self.bytes(sections), offset).map(Some)
    }

    /// Return one logical operation's function-relative byte offset.
    pub fn operation_offset(
        &self,
        sections: SectionImage<'_>,
        function_index: usize,
        operation: u32,
    ) -> Option<CodeOffset> {
        let function = self.function(sections, function_index)?;

        function.operation(self.operations(sections), operation)
    }

    /// Return one logical operation's provenance.
    pub fn operation_provenance(
        &self,
        sections: SectionImage<'_>,
        function_index: usize,
        operation: u32,
    ) -> Option<ProvenanceId> {
        let function = self.function(sections, function_index)?;

        function.operation_provenance(self.operation_provenances(sections), operation)
    }

    /// Return the logical operation containing one function-relative byte offset.
    pub fn operation_at(
        &self,
        sections: SectionImage<'_>,
        function_index: usize,
        offset: CodeOffset,
    ) -> Option<u32> {
        let function = self.function(sections, function_index)?;
        let code = function.code()?;
        if offset.0 >= code.byte_len {
            return None;
        }

        function.operation_at(self.operations(sections), self.mappings(sections), offset)
    }

    /// Return the provenance containing one function-relative byte offset.
    pub fn provenance_at(
        &self,
        sections: SectionImage<'_>,
        function_index: usize,
        offset: CodeOffset,
    ) -> Option<ProvenanceId> {
        let function = self.function(sections, function_index)?;
        let code = function.code()?;
        if offset.0 >= code.byte_len {
            return None;
        }
        function.provenance_at(
            self.operations(sections),
            self.operation_provenances(sections),
            self.mappings(sections),
            offset,
        )
    }
}

/// Linked bytecode under construction.
#[derive(Clone, Debug, Default)]
pub struct CodeBuilder {
    /// Physical functions in Program function order.
    functions: Vec<Function>,
    /// Physical frame maps in canonical frame state order.
    frames: Vec<FrameMap>,
    /// Flattened register spans referenced by frame maps.
    registers: Vec<RegisterSpan>,
    /// Function-relative byte offsets of logical operations.
    operations: Vec<CodeOffset>,
    /// Provenance parallel to logical operation offsets.
    operation_provenances: Vec<ProvenanceId>,
    /// Explicit physical mappings.
    mappings: Vec<Mapping>,
    /// Contiguous linked instruction bytes.
    code: Vec<u8>,
}

impl CodeBuilder {
    /// Create an empty linked bytecode builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set physical functions in Program function order.
    pub fn functions(mut self, functions: impl IntoIterator<Item = Function>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Set physical frame maps in canonical frame state order.
    pub fn frames(mut self, frames: impl IntoIterator<Item = FrameMap>) -> Self {
        self.frames = frames.into_iter().collect();

        self
    }

    /// Set flattened register spans referenced by frame maps.
    pub fn registers(mut self, registers: impl IntoIterator<Item = RegisterSpan>) -> Self {
        self.registers = registers.into_iter().collect();

        self
    }

    /// Set logical operation offsets.
    pub fn operations(
        mut self,
        operations: impl IntoIterator<Item = (CodeOffset, ProvenanceId)>,
    ) -> Self {
        (self.operations, self.operation_provenances) = operations.into_iter().unzip();

        self
    }

    /// Set explicit physical mappings.
    pub fn mappings(mut self, mappings: impl IntoIterator<Item = Mapping>) -> Self {
        self.mappings = mappings.into_iter().collect();

        self
    }

    /// Set contiguous linked instruction bytes.
    pub fn code(mut self, code: impl Into<Vec<u8>>) -> Self {
        self.code = code.into();

        self
    }

    /// Build linked executable bytecode in Program sections.
    pub fn build(self, sections: &mut SectionBuilder) -> Code {
        Code {
            functions: sections.insert(self.functions),
            frames: sections.insert(self.frames),
            registers: sections.insert(self.registers),
            operations: sections.insert(self.operations),
            operation_provenances: sections.insert(self.operation_provenances),
            mappings: sections.insert(self.mappings),
            code: sections.insert(self.code),
        }
    }
}

/// One contiguous bytecode function range.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct CodeRange {
    /// The byte offset from the containing code section.
    pub byte_offset: u32,
    /// The function byte length.
    pub byte_len: u32,
}

/// One physical bytecode position mapped to a logical operation and provenance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Mapping {
    /// The generated function-relative byte offset.
    pub offset: CodeOffset,
    /// The mapped logical operation index.
    pub operation: u32,
    /// The mapped provenance.
    pub provenance: ProvenanceId,
}

impl Mapping {
    /// Create one physical bytecode mapping.
    pub const fn new(offset: CodeOffset, operation: u32, provenance: ProvenanceId) -> Self {
        Self {
            offset,
            operation,
            provenance,
        }
    }
}

impl CodeRange {
    /// Validate this byte range against its containing code section.
    pub fn validate(self, byte_len: usize) -> Result<(), SectionImageError> {
        let Some(end) = self.byte_offset.checked_add(self.byte_len) else {
            return Err(SectionImageError::InvalidRange);
        };
        if end as usize > byte_len {
            return Err(SectionImageError::InvalidRange);
        }

        Ok(())
    }

    /// Borrow this range from its containing code section.
    pub fn slice(self, bytes: &[u8]) -> &[u8] {
        let start = self.byte_offset as usize;
        let end = start + self.byte_len as usize;

        &bytes[start..end]
    }

    /// Iterate over this function's instructions.
    pub fn instructions(self, bytes: &[u8]) -> Instructions<'_> {
        Instructions::new(self.slice(bytes))
    }

    /// Read one instruction at a function-relative byte offset.
    pub fn instruction(self, bytes: &[u8], offset: CodeOffset) -> Result<Instruction<'_>, Error> {
        let start = self.byte_offset as usize + offset.index();
        let end = self.byte_offset as usize + self.byte_len as usize;
        let Some(bytes) = bytes.get(start..end) else {
            return Err(Error::TruncatedInstruction);
        };

        Instruction::read(bytes)
    }
}

/// A byte offset inside one encoded bytecode function.
#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct CodeOffset(pub u32);

impl CodeOffset {
    /// Return this offset as a byte index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

const _: () = assert!(size_of::<Code>() == 112);
const _: () = assert!(size_of::<CodeRange>() == 8);
const _: () = assert!(size_of::<Mapping>() == 12);
const _: () = assert!(size_of::<CodeOffset>() == 4);
