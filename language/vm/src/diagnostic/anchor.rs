use serde::{Deserialize, Serialize};
use tspp_bytecode::CodeOffset;
use tspp_program::{FunctionId, ProgramPoint};
use tspp_serde::Reflect;

/// One executable location attached to a VM diagnostic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum DiagnosticAnchor {
    /// No executable location is available.
    None,
    /// One linked program operation.
    Point(ProgramPoint),
}

/// One frame in a diagnostic call stack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct StackTraceFrame {
    /// The function being executed.
    pub function: FunctionId,
    /// The byte offset of the current instruction.
    pub code_offset: CodeOffset,
    /// The function name when retained by the Program.
    pub function_name: Option<String>,
}
