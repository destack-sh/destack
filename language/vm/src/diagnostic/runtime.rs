use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{DiagnosticAnchor, Error, StackTraceFrame};

/// A runtime error with call stack and location information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RuntimeError {
    /// The underlying error.
    pub error: Error,
    /// The call stack at the time of the error.
    pub stack: Vec<StackTraceFrame>,
    /// The location where the error occurred.
    pub anchor: DiagnosticAnchor,
}

impl RuntimeError {
    /// Create a new runtime error.
    pub fn new(error: Error) -> Self {
        Self {
            error,
            stack: Vec::new(),
            anchor: DiagnosticAnchor::None,
        }
    }

    /// Add call stack information.
    pub fn with_call_stack(mut self, stack: Vec<StackTraceFrame>) -> Self {
        self.stack = stack;
        self
    }

    /// Add location information.
    pub fn with_anchor(mut self, anchor: DiagnosticAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Format a stack trace for display.
    pub fn format_stack_trace(&self) -> String {
        if self.stack.is_empty() {
            return String::new();
        }

        let mut trace = String::from("\nStack trace:\n");
        for (i, frame) in self.stack.iter().rev().enumerate() {
            let name = frame.function_name.as_deref().unwrap_or("<anonymous>");
            trace.push_str(&format!("  {i}: {name} (block {:?})\n", frame.block));
        }
        trace
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.error, self.format_stack_trace())
    }
}

impl std::error::Error for RuntimeError {}

impl From<Error> for RuntimeError {
    fn from(error: Error) -> Self {
        Self::new(error)
    }
}

/// Result type for VM operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Result type for runtime operations that include stack traces.
pub type RuntimeResult<T> = std::result::Result<T, RuntimeError>;
