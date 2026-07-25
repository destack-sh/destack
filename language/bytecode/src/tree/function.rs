use destack_core::{EntryRange, Optional, SectionEntry};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{CodeOffset, CodeRange, FrameMap, RegisterSpan, TypeId};

/// The execution form of one bytecode function.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct Coroutine(u8);

impl Coroutine {
    /// An ordinary function that cannot suspend.
    pub const NONE: Self = Self(0);
    /// An asynchronous function that may await.
    pub const ASYNC: Self = Self(1);
    /// A generator function that may yield.
    pub const GENERATOR: Self = Self(2);
    /// An asynchronous generator that may await and yield.
    pub const ASYNC_GENERATOR: Self = Self(3);

    /// Select one execution form from TS-compatible modifiers.
    pub const fn new(is_async: bool, is_generator: bool) -> Self {
        match (is_async, is_generator) {
            (false, false) => Self::NONE,
            (true, false) => Self::ASYNC,
            (false, true) => Self::GENERATOR,
            (true, true) => Self::ASYNC_GENERATOR,
        }
    }

    /// Return whether this function may await.
    pub const fn is_async(self) -> bool {
        matches!(self, Self::ASYNC | Self::ASYNC_GENERATOR)
    }

    /// Return whether this function may yield.
    pub const fn is_generator(self) -> bool {
        matches!(self, Self::GENERATOR | Self::ASYNC_GENERATOR)
    }
}

/// One physical bytecode function.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Function {
    /// The encoded function body when this object defines the function.
    pub code: Optional<CodeRange>,
    /// Physical entry parameters in declaration order.
    pub parameters: EntryRange<Parameter>,
    /// Physical frame maps used by this function.
    pub frames: EntryRange<FrameMap>,
    /// Function-relative byte offsets of logical operations.
    pub operations: EntryRange<CodeOffset>,
    /// The function result type.
    pub result: TypeId,
    /// The number of 64-bit words in the register file.
    pub register_count: u16,
    /// The function's coroutine execution form.
    pub coroutine: Coroutine,
    /// Reserved function byte.
    reserved: u8,
    /// The number of function-local profile counters.
    pub counter_count: u32,
    /// The number of function-local profile samplers.
    pub sampler_count: u32,
}

impl Function {
    /// Create one imported physical function declaration.
    pub const fn declaration(
        parameters: EntryRange<Parameter>,
        result: TypeId,
        coroutine: Coroutine,
    ) -> Self {
        Self::new(
            Optional::none(),
            parameters,
            EntryRange::empty(),
            EntryRange::empty(),
            result,
            0,
            coroutine,
            0,
            0,
        )
    }

    /// Create one physical bytecode function.
    pub const fn new(
        code: Optional<CodeRange>,
        parameters: EntryRange<Parameter>,
        frames: EntryRange<FrameMap>,
        operations: EntryRange<CodeOffset>,
        result: TypeId,
        register_count: u16,
        coroutine: Coroutine,
        counter_count: u32,
        sampler_count: u32,
    ) -> Self {
        Self {
            code,
            parameters,
            frames,
            operations,
            result,
            register_count,
            coroutine,
            reserved: 0,
            counter_count,
            sampler_count,
        }
    }

    /// Return this function's encoded code range when defined.
    pub fn code(&self) -> Option<CodeRange> {
        self.code.get()
    }

    /// Return physical entry parameters in declaration order.
    pub fn parameters<'a>(&self, parameters: &'a [Parameter]) -> &'a [Parameter] {
        self.parameters.slice(parameters)
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

/// One physical bytecode function parameter.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Parameter {
    /// The entry registers containing the parameter value.
    pub registers: RegisterSpan,
    /// The parameter type identity.
    pub ty: TypeId,
}

impl Parameter {
    /// Create one physical bytecode function parameter.
    pub const fn new(registers: RegisterSpan, ty: TypeId) -> Self {
        Self { registers, ty }
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

const _: () = assert!(size_of::<Function>() == 56);
const _: () = assert!(size_of::<Parameter>() == 8);
const _: () = assert!(size_of::<Coroutine>() == 1);
const _: () = assert!(size_of::<FunctionId>() == 4);
const _: () = assert!(size_of::<CounterId>() == 4);
const _: () = assert!(size_of::<SamplerId>() == 4);
