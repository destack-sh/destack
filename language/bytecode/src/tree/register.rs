use std::mem::size_of;

use destack_core::{EntryRange, SectionEntry, SectionImageError};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Physical bytecode locations for one canonical frame state.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameMap {
    /// Physical register spans retained by this state.
    pub registers: EntryRange<RegisterSpan>,
}

impl FrameMap {
    /// Create one physical frame map.
    pub const fn new(registers: EntryRange<RegisterSpan>) -> Self {
        Self { registers }
    }

    /// Validate this frame map against the shared register column.
    pub(crate) fn validate(self, registers: usize) -> Result<(), SectionImageError> {
        self.registers.validate(registers)
    }

    /// Return the physical register spans retained by this map.
    pub fn registers<'a>(&self, registers: &'a [RegisterSpan]) -> &'a [RegisterSpan] {
        self.registers.slice(registers)
    }
}

/// One contiguous span of 64-bit bytecode registers.
#[repr(C)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct RegisterSpan {
    /// The first register word.
    pub start: RegisterId,
    /// The number of register words.
    pub word_count: u16,
}

impl RegisterSpan {
    /// Create one contiguous register span.
    pub const fn new(start: RegisterId, word_count: u16) -> Self {
        Self { start, word_count }
    }

    /// Create an empty register span.
    pub const fn empty() -> Self {
        Self::new(RegisterId(0), 0)
    }

    /// Return the exclusive physical register end.
    pub const fn end(self) -> u32 {
        self.start.0 as u32 + self.word_count as u32
    }

    /// Return whether this span contains one physical register word.
    pub const fn contains(self, register: RegisterId) -> bool {
        let register = register.0 as u32;

        register >= self.start.0 as u32 && register < self.end()
    }

    /// Return whether this span intersects another physical register span.
    pub const fn intersects(self, other: Self) -> bool {
        (self.start.0 as u32) < other.end() && (other.start.0 as u32) < self.end()
    }
}

/// One physical bytecode register word.
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

const _: () = assert!(size_of::<FrameMap>() == 8);
const _: () = assert!(size_of::<RegisterSpan>() == 4);
const _: () = assert!(size_of::<RegisterId>() == 2);
