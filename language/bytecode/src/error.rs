use std::fmt;

/// A malformed bytecode instruction stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The instruction header or body is truncated.
    TruncatedInstruction,
    /// The instruction byte length is invalid.
    InvalidInstructionLength(usize),
    /// The encoded operand bytes are not word-aligned.
    UnalignedOperands(usize),
    /// One instruction exceeds the encoded length range.
    InstructionTooLarge(usize),
    /// The instruction opcode is not defined by the bytecode ISA.
    InvalidOpcode(u16),
    /// The encoded operands do not match the opcode layout.
    InvalidOperands {
        /// The instruction opcode.
        opcode: u16,
        /// The first invalid operand byte.
        byte_offset: usize,
    },
    /// One function exceeds the bytecode branch displacement range.
    FunctionTooLarge(usize),
}

/// A bytecode result.
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    /// Format this bytecode error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedInstruction => formatter.write_str("truncated bytecode instruction"),
            Self::InvalidInstructionLength(byte_len) => {
                write!(formatter, "invalid bytecode instruction length {byte_len}")
            }
            Self::UnalignedOperands(byte_len) => {
                write!(formatter, "unaligned bytecode operands: {byte_len} bytes")
            }
            Self::InstructionTooLarge(byte_len) => {
                write!(
                    formatter,
                    "bytecode instruction is too large: {byte_len} bytes"
                )
            }
            Self::InvalidOpcode(opcode) => write!(formatter, "invalid bytecode opcode {opcode}"),
            Self::InvalidOperands {
                opcode,
                byte_offset,
            } => write!(
                formatter,
                "invalid bytecode operands for opcode {opcode} at byte {byte_offset}"
            ),
            Self::FunctionTooLarge(byte_len) => {
                write!(
                    formatter,
                    "bytecode function exceeds the i32 branch range: {byte_len} bytes"
                )
            }
        }
    }
}

impl std::error::Error for Error {}
