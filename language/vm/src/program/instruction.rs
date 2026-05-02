use std::fmt;

use super::{Opcode, Payload};

/// One decoded program instruction.
#[derive(Clone, Copy)]
pub(crate) struct Instruction {
    /// The instruction opcode.
    pub opcode: Opcode,
    /// The untagged instruction payload.
    payload: Payload,
}

// instruction should fit in 56 bytes
const _: () = assert!(std::mem::size_of::<Instruction>() <= 56);

impl Instruction {
    /// Create one instruction with a typed payload.
    #[inline(always)]
    pub(crate) fn new<T: Copy>(opcode: Opcode, payload: T) -> Self {
        Self {
            opcode,
            payload: Payload::new(payload),
        }
    }

    /// Borrow this instruction's typed payload.
    #[inline(always)]
    pub(crate) fn payload_as<T>(&self) -> &T {
        self.payload.get_ref()
    }
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instruction")
            .field("opcode", &self.opcode)
            .finish()
    }
}
