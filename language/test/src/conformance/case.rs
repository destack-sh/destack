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

/// A discovered conformance case with metadata.
#[derive(Debug, Clone)]
pub struct Case {
    /// Case name relative to the suite root.
    pub name: String,
    /// File type for parsing.
    pub file_type: FileType,
    /// Whether this case expects an error outcome.
    pub expect_error: bool,
    /// Optional expected output source for parity checks.
    pub expected_output: ExpectedOutput,
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
    /// Create a case that should pass with no parse errors.
    pub fn pass(name: impl Into<String>, file_type: FileType) -> Self {
        Self {
            name: name.into(),
            file_type,
            expect_error: false,
            expected_output: ExpectedOutput::None,
        }
    }

    /// Create a case that should fail with parse errors.
    pub fn fail(name: impl Into<String>, file_type: FileType) -> Self {
        Self {
            name: name.into(),
            file_type,
            expect_error: true,
            expected_output: ExpectedOutput::None,
        }
    }

    /// Attach expected output metadata to this case.
    pub fn with_expected_output(mut self, expected_output: ExpectedOutput) -> Self {
        self.expected_output = expected_output;
        self
    }

    /// Infer the file type from one case name.
    pub fn file_type_from_name(name: &str) -> FileType {
        if name.ends_with(".tsx") {
            FileType::TypeScriptXml
        } else if name.ends_with(".ts") {
            FileType::TypeScript
        } else if name.ends_with(".jsx") {
            FileType::JavaScriptXml
        } else {
            FileType::JavaScript
        }
    }
}
