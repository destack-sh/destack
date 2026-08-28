use destack_core::{EntryRange, Optional, SectionEntry, SectionImageError};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{CodeOffset, CodeRange};

/// One physical bytecode function.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// The encoded function body when this object defines the function.
    pub code: Optional<CodeRange>,
    /// Function-relative byte offsets of logical operations.
    pub operations: EntryRange<CodeOffset>,
    /// The number of 64-bit words in the register file.
    pub register_count: u16,
    /// Explicit initialized entry padding.
    padding: [u8; 2],
}

impl Function {
    /// Create one function without bytecode.
    pub const fn declaration() -> Self {
        Self::new(Optional::none(), EntryRange::empty(), 0)
    }

    /// Create one physical bytecode function.
    pub const fn new(
        code: Optional<CodeRange>,
        operations: EntryRange<CodeOffset>,
        register_count: u16,
    ) -> Self {
        Self {
            code,
            operations,
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
        code_byte_len: usize,
    ) -> Result<(), SectionImageError> {
        if self.padding != [0; 2] {
            return Err(SectionImageError::InvalidEntry);
        }
        self.operations.validate(operations.len())?;

        let operations = self.operations(operations);
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
            }
            None if !operations.is_empty() => {
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

    /// Return one logical operation's function-relative byte offset.
    pub fn operation(&self, operations: &[CodeOffset], operation: u32) -> Option<CodeOffset> {
        self.operations(operations).get(operation as usize).copied()
    }

    /// Return the logical operation containing one physical bytecode offset.
    pub fn operation_at(&self, operations: &[CodeOffset], offset: CodeOffset) -> Option<u32> {
        let operations = self.operations(operations);
        let index = match operations.binary_search(&offset) {
            Ok(index) => index,
            Err(0) => return None,
            Err(index) => index - 1,
        };

        Some(index as u32)
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

const _: () = assert!(size_of::<Function>() == 24);
const _: () = assert!(size_of::<FunctionId>() == 4);
const _: () = assert!(size_of::<CounterId>() == 4);
const _: () = assert!(size_of::<SamplerId>() == 4);
