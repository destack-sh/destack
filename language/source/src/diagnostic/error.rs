use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::FileId;

/// Error produced while annotating one source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotateError {
    /// The annotation file does not match the span file.
    FileMismatch {
        /// The file passed to the renderer.
        source_file: FileId,
        /// The file carried by the span.
        span_file: FileId,
    },
    /// The span start is outside the source file.
    InvalidSpanStart {
        /// The source file.
        file: FileId,
        /// The invalid byte offset.
        offset: u32,
    },
    /// The span end is outside the source file.
    InvalidSpanEnd {
        /// The source file.
        file: FileId,
        /// The invalid byte offset.
        offset: u32,
    },
    /// The requested line has no source text.
    MissingLineText {
        /// The source file.
        file: FileId,
        /// The missing line index.
        line: u32,
    },
    /// The requested line has no span.
    MissingLineSpan {
        /// The source file.
        file: FileId,
        /// The missing line index.
        line: u32,
    },
    /// The visible source slice does not land on valid UTF-8 boundaries.
    InvalidVisibleSlice {
        /// The source file.
        file: FileId,
        /// The slice start within the line.
        start: u32,
        /// The slice end within the line.
        end: u32,
    },
}

impl Display for AnnotateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileMismatch {
                source_file,
                span_file,
            } => write!(
                formatter,
                "diagnostic span file {span_file:?} does not match source file {source_file:?}"
            ),
            Self::InvalidSpanStart { file, offset } => {
                write!(
                    formatter,
                    "diagnostic span start {offset} is outside file {file:?}"
                )
            }
            Self::InvalidSpanEnd { file, offset } => {
                write!(
                    formatter,
                    "diagnostic span end {offset} is outside file {file:?}"
                )
            }
            Self::MissingLineText { file, line } => {
                write!(formatter, "diagnostic line {line} is outside file {file:?}")
            }
            Self::MissingLineSpan { file, line } => {
                write!(
                    formatter,
                    "diagnostic line {line} has no span in file {file:?}"
                )
            }
            Self::InvalidVisibleSlice { file, start, end } => write!(
                formatter,
                "diagnostic visible slice {start}..{end} is not valid UTF-8 in file {file:?}"
            ),
        }
    }
}

impl Error for AnnotateError {}

/// Error produced while rendering diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticRenderError {
    /// A diagnostic references a file that cannot be loaded.
    MissingFile {
        /// The missing file.
        file: FileId,
    },
    /// One source annotation could not be rendered.
    Annotate {
        /// The annotation error.
        error: AnnotateError,
    },
    /// A suggestion edit is attached to the wrong file.
    EditFileMismatch {
        /// The file being edited.
        file: FileId,
        /// The file carried by the edit span.
        edit_file: FileId,
    },
    /// Suggestion edits overlap or are not ordered.
    OverlappingEdits {
        /// The edited file.
        file: FileId,
    },
    /// A suggestion edit span is outside the file.
    EditOutsideFile {
        /// The edited file.
        file: FileId,
        /// The edit start byte offset.
        start: u32,
        /// The edit end byte offset.
        end: u32,
        /// The file length in bytes.
        len: usize,
    },
    /// A suggestion edit span does not land on UTF-8 boundaries.
    EditBoundary {
        /// The edited file.
        file: FileId,
        /// The edit start byte offset.
        start: u32,
        /// The edit end byte offset.
        end: u32,
    },
}

impl Display for DiagnosticRenderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFile { file } => {
                write!(formatter, "missing diagnostic file {file:?}")
            }
            Self::Annotate { error } => Display::fmt(error, formatter),
            Self::EditFileMismatch { file, edit_file } => write!(
                formatter,
                "diagnostic suggestion edit file {edit_file:?} does not match {file:?}"
            ),
            Self::OverlappingEdits { file } => {
                write!(
                    formatter,
                    "diagnostic suggestion edits overlap or are out of order in file {file:?}"
                )
            }
            Self::EditOutsideFile {
                file,
                start,
                end,
                len,
            } => write!(
                formatter,
                "diagnostic suggestion edit span {start}..{end} is outside file {file:?} with length {len}"
            ),
            Self::EditBoundary { file, start, end } => write!(
                formatter,
                "diagnostic suggestion edit span {start}..{end} is not on UTF-8 boundaries in file {file:?}"
            ),
        }
    }
}

impl Error for DiagnosticRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Annotate { error } => Some(error),
            _ => None,
        }
    }
}

impl From<AnnotateError> for DiagnosticRenderError {
    fn from(error: AnnotateError) -> Self {
        Self::Annotate { error }
    }
}
