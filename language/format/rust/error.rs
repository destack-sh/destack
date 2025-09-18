use std::error::Error;

use crate::{GroupId, TagKind};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
/// Series of errors encountered during formatting
pub enum FormatError {
    /// In case a node can't be formatted because it either misses a require child element or
    /// a child is present that should not (e.g. a trailing comma after a rest element).
    SyntaxError { message: &'static str },
    /// In case range formatting failed because the provided range was larger
    /// than the formatted syntax tree
    RangeError { input: TextRange, tree: TextRange },

    /// In case printing the document failed because it has an invalid structure.
    InvalidDocument(InvalidDocumentError),

    /// Formatting failed because some content encountered a situation where a layout
    /// choice by an enclosing [`crate::Format`] resulted in a poor layout for a child [`crate::Format`].
    ///
    /// It's up to an enclosing [`crate::Format`] to handle the error and pick another layout.
    /// This error should not be raised if there's no outer [`crate::Format`] handling the poor layout error,
    /// avoiding that formatting of the whole document fails.
    PoorLayout,
}

impl Error for FormatError {}

pub type FormatResult<F> = Result<F, FormatError>;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidDocumentError {
    /// Mismatching start/end kinds
    ///
    /// ```plain
    /// StartIndent
    /// ...
    /// EndGroup
    /// ```
    StartEndTagMismatch {
        start_kind: TagKind,
        end_kind: TagKind,
    },

    /// End tag without a corresponding start tag.
    ///
    /// ```plain
    /// Text
    /// EndGroup
    /// ```
    StartTagMissing {
        kind: TagKind,
    },

    /// Expected a specific start tag but instead is:
    /// - at the end of the document
    /// - at another start tag
    /// - at an end tag
    ExpectedStart {
        expected_start: TagKind,
        actual: ActualStart,
    },

    UnknownGroupId {
        group_id: GroupId,
    },
}

impl Error for InvalidDocumentError {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ActualStart {
    /// The actual element is not a tag.
    Content,

    /// The actual element was a start tag of another kind.
    Start(TagKind),

    /// The actual element is an end tag instead of a start tag.
    End(TagKind),

    /// Reached the end of the document
    EndOfDocument,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PrintError {
    InvalidDocument(InvalidDocumentError),
}

impl From<PrintError> for FormatError {
    fn from(error: PrintError) -> Self {
        FormatError::from(&error)
    }
}

impl From<&PrintError> for FormatError {
    fn from(error: &PrintError) -> Self {
        match error {
            PrintError::InvalidDocument(reason) => FormatError::InvalidDocument(*reason),
        }
    }
}

impl Error for PrintError {}

pub type PrintResult<F> = Result<F, PrintError>;
