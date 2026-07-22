use destack_core::{EntryRange, Optional, SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{CodeOffset, CodeRange, FrameSlot, ValueType};

/// One bytecode function declaration or definition.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// The stable function symbol name.
    pub name: StringId,
    /// The physical function body.
    pub body: Body,
    /// Stable hash of the relocatable encoded function body.
    pub code_hash: u64,
    /// The function linkage.
    pub linkage: Linkage,
    /// Reserved function bytes.
    reserved: [u8; 7],
}

/// One executable bytecode function body.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Body {
    /// The physical parameter value types.
    pub parameters: EntryRange<ValueType>,
    /// The physical result value types.
    pub results: EntryRange<ValueType>,
    /// The parameters delivered when this function resumes.
    pub resume_parameters: EntryRange<ValueType>,
    /// The logical value types partitioning the physical register file.
    pub register_types: EntryRange<ValueType>,
    /// The frame slots for this definition.
    pub frame_slots: EntryRange<FrameSlot>,
    /// The function-relative byte offset of each logical operation.
    pub operation_offsets: EntryRange<CodeOffset>,
    /// The hidden callable environment type when present.
    pub environment: Optional<ValueType>,
    /// The encoded function body when this object defines the function.
    pub code: Optional<CodeRange>,
    /// The first dense Program counter assigned to this function.
    pub counter_start: u32,
    /// The number of function-local profile counters.
    pub counter_count: u32,
    /// The first dense Program sampler assigned to this function.
    pub sampler_start: u32,
    /// The number of function-local profile samplers.
    pub sampler_count: u32,
    /// The number of 64-bit words in the register file.
    pub register_count: u16,
}

impl Function {
    /// Create one bytecode function declaration or definition.
    pub const fn new(name: StringId, body: Body, code_hash: u64, linkage: Linkage) -> Self {
        Self {
            name,
            body,
            code_hash,
            linkage,
            reserved: [0; 7],
        }
    }
}

impl Body {
    /// Create one executable bytecode function body.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        parameters: EntryRange<ValueType>,
        results: EntryRange<ValueType>,
        resume_parameters: EntryRange<ValueType>,
        environment: Optional<ValueType>,
        register_count: u16,
        register_types: EntryRange<ValueType>,
        frame_slots: EntryRange<FrameSlot>,
        operation_offsets: EntryRange<CodeOffset>,
        code: Optional<CodeRange>,
        counter_count: u32,
        sampler_count: u32,
    ) -> Self {
        Self {
            parameters,
            results,
            resume_parameters,
            register_types,
            frame_slots,
            operation_offsets,
            environment,
            code,
            counter_start: 0,
            counter_count,
            sampler_start: 0,
            sampler_count,
            register_count,
        }
    }

    /// Return this function's physical parameter types.
    pub fn parameters<'a>(&self, types: &'a [ValueType]) -> &'a [ValueType] {
        self.parameters.slice(types)
    }

    /// Return this function's physical result types.
    pub fn results<'a>(&self, types: &'a [ValueType]) -> &'a [ValueType] {
        self.results.slice(types)
    }

    /// Borrow the parameters delivered when this function resumes.
    pub fn resume_parameters<'a>(&self, types: &'a [ValueType]) -> &'a [ValueType] {
        self.resume_parameters.slice(types)
    }

    /// Return the register file word count.
    pub const fn register_count(&self) -> usize {
        self.register_count as usize
    }

    /// Return the logical value types partitioning this function's register file.
    pub fn register_types<'a>(&self, types: &'a [ValueType]) -> &'a [ValueType] {
        self.register_types.slice(types)
    }

    /// Return this function's frame slots.
    pub fn frame_slots<'a>(&self, slots: &'a [FrameSlot]) -> &'a [FrameSlot] {
        self.frame_slots.slice(slots)
    }

    /// Return this function's logical operation offsets.
    pub fn operation_offsets<'a>(&self, offsets: &'a [CodeOffset]) -> &'a [CodeOffset] {
        self.operation_offsets.slice(offsets)
    }

    /// Return one logical operation's function-relative byte offset.
    pub fn operation_offset(&self, offsets: &[CodeOffset], operation: u32) -> Option<CodeOffset> {
        self.operation_offsets(offsets)
            .get(operation as usize)
            .copied()
    }

    /// Return this function's encoded code range when defined.
    pub fn code(&self) -> Option<CodeRange> {
        self.code.get()
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

/// A bytecode register id.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct RegisterId(pub u16);

impl RegisterId {
    /// Return this id as a dense register index.
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

/// One contiguous bytecode register range.
#[repr(C)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct RegisterRange {
    /// The first register word.
    pub start: RegisterId,
    /// The number of register words.
    pub word_count: u16,
}

impl RegisterRange {
    /// Create one contiguous register range.
    pub const fn new(start: RegisterId, word_count: u16) -> Self {
        Self { start, word_count }
    }

    /// Create an empty register range.
    pub const fn empty() -> Self {
        Self::new(RegisterId(0), 0)
    }

    /// Pack logical values into one contiguous register range.
    pub fn pack(registers: &[RegisterId], types: &[ValueType]) -> Option<Self> {
        if registers.len() != types.len() {
            return None;
        }

        // encode an empty window canonically
        if registers.is_empty() {
            return Some(Self::empty());
        }

        // require every logical value immediately after its predecessor
        let start = registers[0];
        let mut next = u32::from(start.0);
        for (register, ty) in registers.iter().zip(types) {
            if u32::from(register.0) != next {
                return None;
            }
            next += u32::from(ty.word_count());
        }
        let word_count = u16::try_from(next - u32::from(start.0)).ok()?;

        Some(Self::new(start, word_count))
    }
}

/// The linkage of one bytecode symbol.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct Linkage(pub u8);

impl Linkage {
    /// A definition visible only inside its object.
    pub const LOCAL: Self = Self(0);
    /// A definition exported by its object.
    pub const EXPORT: Self = Self(1);
    /// A definition supplied by another object.
    pub const EXTERNAL: Self = Self(2);

    /// Return whether this linkage is defined by the bytecode object format.
    pub const fn is_defined(self) -> bool {
        self.0 <= Self::EXTERNAL.0
    }
}

const _: () = assert!(size_of::<Body>() == 96);
const _: () = assert!(size_of::<Function>() == 120);
const _: () = assert!(size_of::<FunctionId>() == 4);
const _: () = assert!(size_of::<RegisterId>() == 2);
const _: () = assert!(size_of::<CounterId>() == 4);
const _: () = assert!(size_of::<SamplerId>() == 4);
const _: () = assert!(size_of::<RegisterRange>() == 4);
const _: () = assert!(size_of::<Linkage>() == 1);
