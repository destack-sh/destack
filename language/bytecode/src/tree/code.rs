use destack_core::{SectionBuilder, SectionEntry, SectionImage, SectionSlice};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FrameSlot, Function, FunctionType, Instructions, ValueType};

/// Linked executable bytecode stored in Program sections.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Code {
    /// Register calling types referenced by linked functions.
    function_types: SectionSlice<FunctionType>,
    /// Flattened function value types.
    value_types: SectionSlice<ValueType>,
    /// Flattened logical local storage slots.
    frame_slots: SectionSlice<FrameSlot>,
    /// Linked bytecode functions in Program function order.
    functions: SectionSlice<Function>,
    /// Contiguous code bytes for every linked function.
    code: SectionSlice<u8>,
}

impl Code {
    /// Return all linked register calling types.
    pub fn function_types<'a>(&self, sections: SectionImage<'a>) -> &'a [FunctionType] {
        sections.entries(self.function_types)
    }

    /// Return all linked function value types.
    pub fn value_types<'a>(&self, sections: SectionImage<'a>) -> &'a [ValueType] {
        sections.entries(self.value_types)
    }

    /// Return all linked frame slots.
    pub fn frame_slots<'a>(&self, sections: SectionImage<'a>) -> &'a [FrameSlot] {
        sections.entries(self.frame_slots)
    }

    /// Return all linked bytecode functions.
    pub fn functions<'a>(&self, sections: SectionImage<'a>) -> &'a [Function] {
        sections.entries(self.functions)
    }

    /// Return one linked bytecode function by its Program table index.
    pub fn function<'a>(
        &self,
        sections: SectionImage<'a>,
        function_index: usize,
    ) -> Option<&'a Function> {
        self.functions(sections).get(function_index)
    }

    /// Iterate one linked function's instructions by its Program table index.
    pub fn instructions<'a>(
        &self,
        sections: SectionImage<'a>,
        function_index: usize,
    ) -> Option<Instructions<'a>> {
        let function = self.function(sections, function_index)?;
        let code = function.code()?;

        Some(code.instructions(sections.entries(self.code)))
    }
}

/// Linked bytecode under construction.
#[derive(Clone, Debug, Default)]
pub struct CodeBuilder {
    /// Linked register calling types.
    function_types: Vec<FunctionType>,
    /// Flattened function value types.
    value_types: Vec<ValueType>,
    /// Flattened logical local storage slots.
    frame_slots: Vec<FrameSlot>,
    /// Linked functions in Program function order.
    functions: Vec<Function>,
    /// Encoded function bytes.
    code: Vec<u8>,
}

impl CodeBuilder {
    /// Create an empty linked bytecode builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set linked function types.
    pub fn function_types(mut self, entries: impl IntoIterator<Item = FunctionType>) -> Self {
        self.function_types = entries.into_iter().collect();

        self
    }

    /// Set flattened function value types.
    pub fn value_types(mut self, entries: impl IntoIterator<Item = ValueType>) -> Self {
        self.value_types = entries.into_iter().collect();

        self
    }

    /// Set flattened logical local storage slots.
    pub fn frame_slots(mut self, entries: impl IntoIterator<Item = FrameSlot>) -> Self {
        self.frame_slots = entries.into_iter().collect();

        self
    }

    /// Set linked functions in Program function order.
    pub fn functions(mut self, entries: impl IntoIterator<Item = Function>) -> Self {
        self.functions = entries.into_iter().collect();

        self
    }

    /// Set encoded function bytes.
    pub fn code(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.code = bytes.into();

        self
    }

    /// Build linked executable bytecode in Program sections.
    pub fn build(self, sections: &mut SectionBuilder) -> Code {
        let function_types = sections.insert(self.function_types);
        let value_types = sections.insert(self.value_types);
        let frame_slots = sections.insert(self.frame_slots);
        let functions = sections.insert(self.functions);
        let code = sections.insert(self.code);

        Code {
            function_types,
            value_types,
            frame_slots,
            functions,
            code,
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

impl CodeRange {
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
}

/// A byte offset inside one bytecode function.
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

const _: () = assert!(size_of::<Code>() == 80);
const _: () = assert!(size_of::<CodeRange>() == 8);
const _: () = assert!(size_of::<CodeOffset>() == 4);
