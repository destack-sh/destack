use std::path::PathBuf;

use destack_source::FileType;

/// Result of running a single conformance case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseOutcome {
    /// Case passed.
    Passed,
    /// Case failed because parsing failed.
    FailedParse,
    /// Case failed because output mismatched expected output.
    FailedOutput,
    /// Case failed because formatting was not idempotent.
    FailedIdempotence,
    /// Case failed because fixture input could not be read.
    FailedRead,
}

/// Expected parser validity for one conformance source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceValidity {
    /// Source should parse without diagnostics.
    Valid,
    /// Source should be rejected by parser diagnostics.
    Invalid,
}

impl SourceValidity {
    /// Return whether the source is expected to be rejected.
    pub const fn is_invalid(self) -> bool {
        matches!(self, Self::Invalid)
    }
}

/// A discovered conformance case with metadata.
#[derive(Debug, Clone)]
pub struct Case {
    /// Case name relative to the suite root.
    pub name: String,
    /// File type for parsing.
    pub file_type: FileType,
    /// Expected parser validity for the source file.
    pub source_validity: SourceValidity,
    /// Optional expected output source for parity checks.
    pub expected_output: ExpectedOutput,
    /// Whether to require a stable second pass (some cases aren't meaningfully stable).
    pub check_idempotence: bool,
}

/// Expected output source for formatter conformance cases.
#[derive(Debug, Clone, Default)]
pub enum ExpectedOutput {
    /// No external expected output is available.
    #[default]
    None,
    /// Compare output against a plain text file.
    PlainFile(PathBuf),
    /// Compare output against oxfmt style snapshot file sections.
    OxfmtSnapshot(PathBuf),
}

impl Case {
    /// Create a source case that should parse without diagnostics.
    pub fn valid(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            file_type: FileType::Destack,
            source_validity: SourceValidity::Valid,
            expected_output: ExpectedOutput::None,
            check_idempotence: true,
        }
    }

    /// Create a source case that should be rejected by parser diagnostics.
    pub fn invalid(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            file_type: FileType::Destack,
            source_validity: SourceValidity::Invalid,
            expected_output: ExpectedOutput::None,
            check_idempotence: true,
        }
    }

    /// Parse this case as a `.d.ds` declaration source.
    pub fn declaration(mut self) -> Self {
        self.file_type = FileType::DestackDeclaration;

        self
    }

    /// Attach expected output metadata to this case.
    pub fn with_expected_output(mut self, expected_output: ExpectedOutput) -> Self {
        self.expected_output = expected_output;
        self
    }

    /// Disable formatter idempotence checks for this case.
    pub fn without_idempotence(mut self) -> Self {
        self.check_idempotence = false;
        self
    }
}
