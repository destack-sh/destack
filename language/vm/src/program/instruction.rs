use std::fmt;

use super::Op;

/// One decoded program instruction.
#[derive(Clone, Copy)]
pub(crate) struct Instruction {
    /// The instruction operation.
    pub op: Op,
    /// First 32-bit operand.
    pub a: u32,
    /// Second 32-bit operand.
    pub b: u32,
    /// Third 32-bit operand.
    pub c: u32,
    /// Fourth 32-bit operand.
    pub d: u32,
}

// instruction should fit in 20 bytes
const _: () = assert!(std::mem::size_of::<Instruction>() <= 20);

impl Instruction {
    /// Create one instruction.
    #[inline(always)]
    pub(crate) const fn new(op: Op, a: u32, b: u32, c: u32, d: u32) -> Self {
        Self { op, a, b, c, d }
    }
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instruction").field("op", &self.op).finish()
    }
}
