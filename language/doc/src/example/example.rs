use std::fmt::{self, Display, Formatter};
use std::ops::Range;

use tspp_source::DiagnosticSeverity;

/// One checked documentation example: the files of one section and the probes placed on them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    /// The heading path naming the example, such as `Structs > Copies`.
    pub name: String,
    /// The files the example's blocks declare, in block order.
    pub files: Vec<ExampleFile>,
}

/// One source file of an example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleFile {
    /// The package-relative path.
    pub path: String,
    /// The source without annotations.
    pub source: String,
    /// The offset in `source` where the shown part starts, after any cut.
    pub shown: u32,
    /// The probes placed on this file, in source order.
    pub probes: Vec<Probe>,
    /// The byte range of the block content in its document.
    pub block: Range<usize>,
}

/// One expectation placed on a source range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The byte range in the file source the probe points at.
    pub range: Range<u32>,
    /// What the toolchain must report at the range.
    pub expectation: Expectation,
}

/// What one probe expects the toolchain to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expectation {
    /// The type the hover query reports, written `^? Type`.
    Type(String),
    /// One diagnostic, written `^^^ error[code]: message`.
    Diagnostic {
        /// The diagnostic severity.
        severity: DiagnosticSeverity,
        /// The diagnostic code.
        code: String,
        /// The rendered diagnostic message.
        message: String,
    },
    /// The displayed values of an expression statement, written `// => value`.
    Value(String),
}

/// One malformed example annotation or block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleError {
    /// The byte offset in the document.
    pub offset: usize,
    /// The failure description.
    pub message: String,
}

impl ExampleError {
    /// Create one error at a document offset.
    pub(crate) fn new(offset: usize, message: &str) -> Self {
        Self {
            offset,
            message: message.to_string(),
        }
    }
}

impl Display for ExampleError {
    /// Format the failure with its document offset.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for ExampleError {}
