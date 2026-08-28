use std::fmt;

use serde::{Deserialize, Serialize};

/// A malformed bytecode instruction stream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Error {
    /// The instruction destinations do not match its opcode.
    InvalidResults,
    /// The instruction destinations are not one contiguous register range.
    NoncontiguousResults,
    /// No logical operation is available to mark.
    MissingOperation,
    /// A logical operation id does not exist in this function.
    InvalidOperation(u32),
    /// A branch label is not defined in dense declaration order.
    InvalidLabel(u32),
    /// A branch references an undefined label.
    UnknownLabel(u32),
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
    /// One encoded instruction operand is not defined by the bytecode ISA.
    InvalidOperand,
    /// One variable operand list exceeds its encoded count range.
    TooManyOperands(usize),
    /// One function exceeds the bytecode branch displacement range.
    FunctionTooLarge(usize),
    /// One function exceeds the encoded register count range.
    RegisterFileTooLarge(u32),
}

/// A bytecode result.
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    /// Format this bytecode error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidResults => {
                formatter.write_str("instruction results do not match its opcode")
            }
            Self::NoncontiguousResults => {
                formatter.write_str("instruction results are not contiguous")
            }
            Self::MissingOperation => formatter.write_str("no bytecode operation is active"),
            Self::InvalidOperation(operation) => {
                write!(formatter, "invalid bytecode operation {operation}")
            }
            Self::InvalidLabel(label) => {
                write!(
                    formatter,
                    "bytecode label b{label} is not dense and ordered"
                )
            }
            Self::UnknownLabel(label) => write!(formatter, "unknown bytecode label b{label}"),
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
            Self::InvalidOperand => formatter.write_str("invalid bytecode instruction operand"),
            Self::TooManyOperands(count) => {
                write!(
                    formatter,
                    "bytecode operand list is too large: {count} entries"
                )
            }
            Self::FunctionTooLarge(byte_len) => {
                write!(
                    formatter,
                    "bytecode function exceeds the i32 branch range: {byte_len} bytes"
                )
            }
            Self::RegisterFileTooLarge(register_count) => {
                write!(
                    formatter,
                    "bytecode function register file is too large: {register_count} words"
                )
            }
        }
    }
}

impl std::error::Error for Error {}
