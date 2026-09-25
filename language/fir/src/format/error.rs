use std::error::Error;

use tspp_source::Span;

use crate::format::{FormatTagKind, GroupId};

/// Requested formatted output bytes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RequestedOutputBytes {
    /// A known output byte count.
    Count(usize),
    /// The requested output byte count overflowed `usize`.
    Overflow,
}

impl std::fmt::Display for RequestedOutputBytes {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestedOutputBytes::Count(count) => std::write!(fmt, "{count} requested bytes"),
            RequestedOutputBytes::Overflow => std::write!(fmt, "an overflowing byte count"),
        }
    }
}

/// One failure encountered while constructing or printing FIR.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum FormatError {
    /// The source tree cannot be formatted because its structure is invalid.
    SyntaxError {
        /// The static source-tree validation message.
        message: &'static str,
    },
    /// Range formatting failed because the provided range was larger
    /// than the formatted syntax tree.
    RangeError {
        /// The requested source range.
        input: Span,
        /// The complete source-tree range.
        tree: Span,
    },
    /// Source text was unavailable for a source slice.
    SourceTextUnavailable {
        /// The unavailable source range.
        span: Span,
    },
    /// Formatted output exceeded the configured byte limit.
    OutputTooLarge {
        /// The configured output byte limit.
        max_output_bytes: u32,
        /// The requested output size.
        requested_bytes: RequestedOutputBytes,
    },
    /// Printing the document failed because it has an invalid structure.
    InvalidDocument(InvalidDocumentError),
    /// An enclosing format implementation selected a poor layout for nested content.
    ///
    /// An enclosing [`crate::format::Format`] may handle this error by selecting another layout.
    PoorLayout,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::SyntaxError { message } => {
                std::write!(fmt, "syntax error: {message}")
            }
            FormatError::RangeError { input, tree } => std::write!(
                fmt,
                "formatting range {input:?} is larger than syntax tree {tree:?}"
            ),
            FormatError::SourceTextUnavailable { span } => {
                std::write!(fmt, "source text is unavailable for span {span:?}")
            }
            FormatError::OutputTooLarge {
                max_output_bytes,
                requested_bytes,
            } => std::write!(
                fmt,
                "formatted output exceeded byte limit {max_output_bytes} with {requested_bytes}"
            ),
            FormatError::InvalidDocument(error) => std::write!(fmt, "invalid document: {error}"),
            FormatError::PoorLayout => {
                std::write!(
                    fmt,
                    "poor layout: the formatter wasn't able to pick a good layout"
                )
            }
        }
    }
}

impl Error for FormatError {}

/// The result of one FIR construction operation.
pub type FormatResult<F> = Result<F, FormatError>;

/// One structural error in an encoded FIR document.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidDocumentError {
    /// One end tag does not match the active start tag.
    ///
    /// ```plain
    /// StartIndent
    /// ...
    /// EndGroup
    /// ```
    StartEndTagMismatch {
        /// The active structural scope.
        start_kind: FormatTagKind,
        /// The encountered end tag.
        end_kind: FormatTagKind,
    },

    /// End tag without a corresponding start tag.
    ///
    /// ```plain
    /// Text
    /// EndGroup
    /// ```
    StartTagMissing {
        /// The unmatched end tag kind.
        kind: FormatTagKind,
    },

    /// A start tag without its corresponding end tag.
    EndTagMissing {
        /// The unclosed structural kind.
        kind: FormatTagKind,
    },

    /// An expected start tag is absent.
    ExpectedStart {
        /// The expected structural kind.
        expected_start: FormatTagKind,
        /// The instruction found instead.
        actual: ActualStart,
    },

    /// An instruction references a group that has not been printed.
    UnknownGroupId {
        /// The unknown document-local group identifier.
        group_id: GroupId,
    },
}

impl std::fmt::Display for InvalidDocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidDocumentError::StartEndTagMismatch {
                start_kind,
                end_kind,
            } => {
                std::write!(
                    f,
                    "expected end tag of kind {start_kind:?} but found {end_kind:?}"
                )
            }
            InvalidDocumentError::StartTagMissing { kind } => {
                std::write!(f, "end tag of kind {kind:?} without matching start tag")
            }
            InvalidDocumentError::EndTagMissing { kind } => {
                std::write!(f, "start tag of kind {kind:?} without matching end tag")
            }
            InvalidDocumentError::ExpectedStart {
                expected_start,
                actual,
            } => match actual {
                ActualStart::EndOfDocument => {
                    std::write!(
                        f,
                        "expected start tag of kind {expected_start:?} at the end of the document"
                    )
                }
                ActualStart::Start(start) => {
                    std::write!(
                        f,
                        "expected start tag of kind {expected_start:?} but found start tag of kind {start:?}"
                    )
                }
                ActualStart::End(end) => {
                    std::write!(
                        f,
                        "expected start tag of kind {expected_start:?} but found end tag of kind {end:?}"
                    )
                }
                ActualStart::Content => {
                    std::write!(
                        f,
                        "expected start tag of kind {expected_start:?} but found content"
                    )
                }
            },
            InvalidDocumentError::UnknownGroupId { group_id } => {
                std::write!(
                    f,
                    "unknown group id {group_id:?}: the group must precede every instruction that references it"
                )
            }
        }
    }
}

impl Error for InvalidDocumentError {}

/// The instruction encountered where a structural start tag was expected.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ActualStart {
    /// The actual instruction is content.
    Content,

    /// The actual instruction starts another structural kind.
    Start(FormatTagKind),

    /// The actual instruction ends a structural kind.
    End(FormatTagKind),

    /// Reached the end of the document.
    EndOfDocument,
}

/// One failure encountered while printing FIR.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PrintError {
    /// Source text was unavailable for a source slice.
    SourceTextUnavailable {
        /// The unavailable source range.
        span: Span,
    },
    /// Formatted output exceeded the configured byte limit.
    OutputTooLarge {
        /// The configured output byte limit.
        max_output_bytes: u32,
        /// The requested output size.
        requested_bytes: RequestedOutputBytes,
    },
    /// Printing failed because the document has an invalid structure.
    InvalidDocument(InvalidDocumentError),
    /// Repeated measurement of the same layout produced inconsistent results.
    UnstableLayout,
}

impl From<PrintError> for FormatError {
    fn from(error: PrintError) -> Self {
        FormatError::from(&error)
    }
}

impl From<&PrintError> for FormatError {
    fn from(error: &PrintError) -> Self {
        match error {
            PrintError::SourceTextUnavailable { span } => {
                FormatError::SourceTextUnavailable { span: *span }
            }
            PrintError::OutputTooLarge {
                max_output_bytes,
                requested_bytes,
            } => FormatError::OutputTooLarge {
                max_output_bytes: *max_output_bytes,
                requested_bytes: *requested_bytes,
            },
            PrintError::InvalidDocument(reason) => FormatError::InvalidDocument(*reason),
            PrintError::UnstableLayout => FormatError::PoorLayout,
        }
    }
}

impl std::fmt::Display for PrintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrintError::SourceTextUnavailable { span } => {
                std::write!(f, "source text is unavailable for span {span:?}")
            }
            PrintError::OutputTooLarge {
                max_output_bytes,
                requested_bytes,
            } => {
                std::write!(
                    f,
                    "formatted output exceeded byte limit {max_output_bytes} with {requested_bytes}"
                )
            }
            PrintError::InvalidDocument(inner) => {
                std::write!(f, "invalid document: {inner}")
            }
            PrintError::UnstableLayout => {
                std::write!(
                    f,
                    "repeated layout measurement produced inconsistent results"
                )
            }
        }
    }
}

impl Error for PrintError {}

/// The result of one FIR print operation.
pub type PrintResult<F> = Result<F, PrintError>;
