use std::fmt;

use super::{Opcode, Operands};

/// One decoded program instruction.
#[derive(Clone)]
pub(crate) struct Instruction {
    /// The instruction opcode.
    pub opcode: Opcode,
    /// The encoded operands for this opcode.
    pub operands: Operands,
}

// instruction should fit in 56 bytes
const _: () = assert!(std::mem::size_of::<Instruction>() <= 56);

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instruction")
            .field("opcode", &self.opcode)
            .field("operands", &self.operands)
            .finish()
    }
}
