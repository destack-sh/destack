use destack_core::{EntryRange, Optional, SectionEntry};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{CodeOffset, CodeRange, FrameMap};

/// One physical bytecode function.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// The encoded function body when this object defines the function.
    pub code: Optional<CodeRange>,
    /// Physical frame maps used by this function.
    pub frames: EntryRange<FrameMap>,
    /// Function-relative byte offsets of logical operations.
    pub operations: EntryRange<CodeOffset>,
    /// The number of 64-bit words in the register file.
    pub register_count: u16,
    /// Reserved function bytes.
    reserved: [u8; 2],
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
        frames: EntryRange<FrameMap>,
        operations: EntryRange<CodeOffset>,
        register_count: u16,
    ) -> Self {
        Self {
            code,
            frames,
            operations,
            register_count,
            reserved: [0; 2],
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

    /// Return this function's logical operation offsets.
    pub fn operations<'a>(&self, operations: &'a [CodeOffset]) -> &'a [CodeOffset] {
        self.operations.slice(operations)
    }

    /// Return one logical operation's function-relative byte offset.
    pub fn operation(&self, operations: &[CodeOffset], operation: u32) -> Option<CodeOffset> {
        self.operations(operations).get(operation as usize).copied()
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
