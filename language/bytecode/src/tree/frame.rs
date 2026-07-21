use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{RegisterRange, TypeId};

/// One storage slot in a bytecode frame.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FrameSlot {
    /// The object-local stored type.
    pub ty: TypeId,
    /// The register range saved into this slot at suspension points.
    registers: RegisterRange,
}

impl FrameSlot {
    /// Create one addressable frame slot.
    pub const fn new(ty: TypeId) -> Self {
        Self {
            ty,
            registers: RegisterRange::empty(),
        }
    }

    /// Create one frame slot saved from a contiguous register range.
    pub const fn from_registers(ty: TypeId, registers: RegisterRange) -> Self {
        Self { ty, registers }
    }

    /// Return the register range saved into this slot at suspension points.
    pub const fn registers(self) -> Option<RegisterRange> {
        if self.registers.word_count == 0 {
            None
        } else {
            Some(self.registers)
        }
    }
}

const _: () = assert!(size_of::<FrameSlot>() == 8);
