use destack_core::{EntryRange, Optional, SectionEntry, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{CodeRange, FrameSlot, ValueType};

/// One bytecode function declaration or definition.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// The stable function symbol name.
    pub name: StringId,
    /// The values delivered when this function resumes.
    pub resume_types: EntryRange<ValueType>,
    /// The logical local storage slots for this definition.
    pub frame_slots: EntryRange<FrameSlot>,
    /// The hidden callable environment type when present.
    pub environment: Optional<ValueType>,
    /// Stable hash of the encoded function body.
    pub code_hash: u64,
    /// The encoded function body when this object defines the function.
    pub code: Optional<CodeRange>,
    /// The register calling type.
    pub function_type: FunctionTypeId,
    /// The number of function-local profile counters.
    pub counter_count: u32,
    /// The number of function-local profile samplers.
    pub sampler_count: u32,
    /// The number of 64-bit words in the register file.
    pub register_count: u16,
    /// The function linkage.
    pub linkage: Linkage,
    /// Reserved function byte.
    reserved: u8,
}

impl Function {
    /// Create one bytecode function declaration or definition.
    pub const fn new(
        name: StringId,
        function_type: FunctionTypeId,
        resume_types: EntryRange<ValueType>,
        linkage: Linkage,
        environment: Optional<ValueType>,
        register_count: u16,
        frame_slots: EntryRange<FrameSlot>,
        code: Optional<CodeRange>,
        counter_count: u32,
        sampler_count: u32,
        code_hash: u64,
    ) -> Self {
        Self {
            name,
            resume_types,
            frame_slots,
            environment,
            code_hash,
            code,
            function_type,
            counter_count,
            sampler_count,
            register_count,
            linkage,
            reserved: 0,
        }
    }

    /// Borrow the values delivered when this function resumes.
    pub fn resume_types<'a>(&self, types: &'a [ValueType]) -> &'a [ValueType] {
        self.resume_types.slice(types)
    }

    /// Return the register file word count.
    pub const fn register_count(&self) -> usize {
        self.register_count as usize
    }

    /// Return this function's logical local storage slots.
    pub fn frame_slots<'a>(&self, slots: &'a [FrameSlot]) -> &'a [FrameSlot] {
        self.frame_slots.slice(slots)
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

/// An object-local function type id.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct FunctionTypeId(pub u32);

impl FunctionTypeId {
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

/// The register calling type of one bytecode function.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FunctionType {
    /// The stable type name when explicitly named.
    pub name: Optional<StringId>,
    /// The parameter value representations in call order.
    pub parameters: EntryRange<ValueType>,
    /// The result value representations in return order.
    pub results: EntryRange<ValueType>,
}

impl FunctionType {
    /// Borrow the parameter types.
    pub fn parameters<'a>(&self, types: &'a [ValueType]) -> &'a [ValueType] {
        self.parameters.slice(types)
    }

    /// Borrow the result types.
    pub fn results<'a>(&self, types: &'a [ValueType]) -> &'a [ValueType] {
        self.results.slice(types)
    }
}

const _: () = assert!(size_of::<Function>() == 80);
const _: () = assert!(size_of::<FunctionId>() == 4);
const _: () = assert!(size_of::<FunctionTypeId>() == 4);
const _: () = assert!(size_of::<RegisterId>() == 2);
const _: () = assert!(size_of::<CounterId>() == 4);
const _: () = assert!(size_of::<SamplerId>() == 4);
const _: () = assert!(size_of::<RegisterRange>() == 4);
const _: () = assert!(size_of::<Linkage>() == 1);
const _: () = assert!(size_of::<FunctionType>() == 32);
