use destack_core::{EntryRange, Optional, SectionEntry, SectionImageError};
use destack_serde::Reflect;
use destack_source::ProvenanceId;
use serde::{Deserialize, Serialize};

use crate::{CodeOffset, CodeRange, Mapping};

/// One physical bytecode function.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// The encoded function body when this object defines the function.
    pub code: Optional<CodeRange>,
    /// Function-relative byte offsets of logical operations.
    pub operations: EntryRange<CodeOffset>,
    /// Mappings for physical code emitted under an existing logical operation.
    pub mappings: EntryRange<Mapping>,
    /// The number of 64-bit words in the register file.
    pub register_count: u16,
    /// Explicit initialized entry padding.
    padding: [u8; 2],
}

impl Function {
    /// Create one function without bytecode.
    pub const fn declaration() -> Self {
        Self::new(
            Optional::none(),
            EntryRange::empty(),
            EntryRange::empty(),
            0,
        )
    }

    /// Create one physical bytecode function.
    pub const fn new(
        code: Optional<CodeRange>,
        operations: EntryRange<CodeOffset>,
        mappings: EntryRange<Mapping>,
        register_count: u16,
    ) -> Self {
        Self {
            code,
            operations,
            mappings,
            register_count,
            padding: [0; 2],
        }
    }

    /// Return this function's encoded code range when defined.
    pub fn code(&self) -> Option<CodeRange> {
        self.code.get()
    }

    /// Return the register file word count.
    pub const fn register_count(&self) -> usize {
        self.register_count as usize
    }

    /// Validate this function against its flattened columns.
    pub(crate) fn validate(
        &self,
        operations: &[CodeOffset],
        mappings: &[Mapping],
        code_byte_len: usize,
    ) -> Result<(), SectionImageError> {
        if self.padding != [0; 2] {
            return Err(SectionImageError::InvalidEntry);
        }
        self.operations.validate(operations.len())?;
        self.mappings.validate(mappings.len())?;

        let operations = self.operations(operations);
        let mappings = self.mappings(mappings);
        if !operations.windows(2).all(|pair| pair[0] <= pair[1]) {
            return Err(SectionImageError::InvalidOrder);
        }

        // validate offsets against the optional function body
        match self.code() {
            Some(code) => {
                code.validate(code_byte_len)?;
                if operations
                    .last()
                    .is_some_and(|offset| offset.0 >= code.byte_len)
                {
                    return Err(SectionImageError::InvalidRange);
                }

                // require complete mapping for nonempty code
                let is_mapped = operations.first() == Some(&CodeOffset(0))
                    || mappings
                        .first()
                        .is_some_and(|mapping| mapping.offset == CodeOffset(0));
                if code.byte_len > 0 && !is_mapped {
                    return Err(SectionImageError::InvalidRange);
                }

                // validate explicit physical mappings
                for mapping in mappings {
                    if mapping.offset.0 >= code.byte_len
                        || mapping.operation as usize >= operations.len()
                    {
                        return Err(SectionImageError::InvalidRange);
                    }
                }
                if !mappings
                    .windows(2)
                    .all(|pair| pair[0].offset < pair[1].offset)
                {
                    return Err(SectionImageError::InvalidOrder);
                }
            }
            None if !operations.is_empty() || !mappings.is_empty() => {
                return Err(SectionImageError::InvalidRange);
            }
            None => {}
        }

        Ok(())
    }

    /// Return this function's logical operation offsets.
    pub fn operations<'a>(&self, operations: &'a [CodeOffset]) -> &'a [CodeOffset] {
        self.operations.slice(operations)
    }

    /// Return this function's logical operation provenance.
    pub fn operation_provenances<'a>(&self, provenances: &'a [ProvenanceId]) -> &'a [ProvenanceId] {
        let start = self.operations.start as usize;
        let end = start + self.operations.len as usize;

        &provenances[start..end]
    }

    /// Return this function's explicit physical mappings.
    pub fn mappings<'a>(&self, mappings: &'a [Mapping]) -> &'a [Mapping] {
        self.mappings.slice(mappings)
    }

    /// Return one logical operation's function-relative byte offset.
    pub fn operation(&self, operations: &[CodeOffset], operation: u32) -> Option<CodeOffset> {
        self.operations(operations).get(operation as usize).copied()
    }

    /// Return one logical operation's provenance.
    pub fn operation_provenance(
        &self,
        provenances: &[ProvenanceId],
        operation: u32,
    ) -> Option<ProvenanceId> {
        self.operation_provenances(provenances)
            .get(operation as usize)
            .copied()
    }

    /// Return the logical operation containing one physical bytecode offset.
    pub fn operation_at(
        &self,
        operations: &[CodeOffset],
        mappings: &[Mapping],
        offset: CodeOffset,
    ) -> Option<u32> {
        let operation = self.find_operation(operations, offset);
        let mapping = self.find_mapping(mappings, offset);
        if let Some(mapping) = mapping
            && operation.is_none_or(|(_, operation_offset)| mapping.offset >= operation_offset)
        {
            return Some(mapping.operation);
        }

        operation.map(|(operation, _)| operation)
    }

    /// Return the provenance containing one physical bytecode offset.
    pub fn provenance_at(
        &self,
        operations: &[CodeOffset],
        operation_provenances: &[ProvenanceId],
        mappings: &[Mapping],
        offset: CodeOffset,
    ) -> Option<ProvenanceId> {
        let operation = self.find_operation(operations, offset);
        let mapping = self.find_mapping(mappings, offset);
        if let Some(mapping) = mapping
            && operation.is_none_or(|(_, operation_offset)| mapping.offset >= operation_offset)
        {
            return Some(mapping.provenance);
        }
        let (operation, _) = operation?;

        self.operation_provenance(operation_provenances, operation)
    }

    /// Find the logical operation implied by ordinary operation offsets.
    fn find_operation(
        &self,
        operations: &[CodeOffset],
        offset: CodeOffset,
    ) -> Option<(u32, CodeOffset)> {
        let operations = self.operations(operations);
        let index = operations.partition_point(|candidate| *candidate <= offset);
        let index = index.checked_sub(1)?;

        Some((index as u32, operations[index]))
    }

    /// Find the latest explicit mapping at one physical byte offset.
    fn find_mapping<'a>(&self, mappings: &'a [Mapping], offset: CodeOffset) -> Option<&'a Mapping> {
        let mappings = self.mappings(mappings);
        let index = mappings.partition_point(|mapping| mapping.offset <= offset);

        mappings.get(index.checked_sub(1)?)
    }
}

/// An object-local function id.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct FunctionId(pub u32);

impl FunctionId {
    /// Return this id as a dense object index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// One function-local profile counter id.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct CounterId(pub u32);

impl CounterId {
    /// Return this id as a dense function-local index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// One function-local profile sampler id.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct SamplerId(pub u32);

impl SamplerId {
    /// Return this id as a dense function-local index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

const _: () = assert!(size_of::<Function>() == 32);
const _: () = assert!(size_of::<FunctionId>() == 4);
const _: () = assert!(size_of::<CounterId>() == 4);
const _: () = assert!(size_of::<SamplerId>() == 4);
