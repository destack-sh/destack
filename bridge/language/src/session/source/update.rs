use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use destack_session as session;

use crate::{Revision, RevisionParseError, bridge};

use super::Change;

/// One text range in byte offsets.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

/// One text replacement.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Replaced byte range.
    pub range: TextRange,
    /// Replacement text.
    pub text: String,
}

/// One edit accepted by a session.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Replace or create one text file.
    SetText {
        /// Repository logical path.
        path: String,
        /// Full text content.
        text: String,
    },
    /// Apply text replacements to one tracked text file.
    EditText {
        /// Repository logical path.
        path: String,
        /// Text replacements.
        edits: Vec<TextEdit>,
    },
    /// Replace or create one binary file.
    SetBytes {
        /// Repository logical path.
        path: String,
        /// Full binary content.
        bytes: Vec<u8>,
    },
    /// Remove one file.
    Remove {
        /// Repository logical path.
        path: String,
    },
    /// Move one file.
    Move {
        /// Repository logical path.
        from: String,
        /// Destination repository logical path.
        to: String,
    },
}

/// One committed edit batch.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
    /// Changed files.
    pub changes: Vec<Change>,
}

/// Error returned when a source bridge value cannot become a session value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceBridgeError {
    /// The revision id is invalid.
    Revision(crate::RevisionParseError),
}

impl Commit {
    /// Convert one session commit through one live session.
    pub fn from_session_commit(session: &session::Session, result: session::Commit) -> Self {
        let session::Commit {
            before,
            after,
            changes,
        } = result;
        let changes = changes
            .into_iter()
            .map(|change| Change::from_session_change(session, change))
            .collect();

        Self {
            before: Revision::from_repository(before),
            after: Revision::from_repository(after),
            changes,
        }
    }
}

impl TryFrom<Edit> for session::Edit {
    type Error = SourceBridgeError;

    /// Convert one bridge edit into one session edit.
    fn try_from(edit: Edit) -> Result<Self, Self::Error> {
        match edit {
            Edit::SetText { path, text } => Ok(Self::SetText {
                path: PathBuf::from(path),
                text,
            }),
            Edit::EditText { path, edits } => Ok(Self::EditText {
                path: PathBuf::from(path),
                edits: edits.into_iter().map(session::TextEdit::from).collect(),
            }),
            Edit::SetBytes { path, bytes } => Ok(Self::SetBytes {
                path: PathBuf::from(path),
                bytes,
            }),
            Edit::Remove { path } => Ok(Self::Remove {
                path: PathBuf::from(path),
            }),
            Edit::Move { from, to } => Ok(Self::Move {
                from: PathBuf::from(from),
                to: PathBuf::from(to),
            }),
        }
    }
}

impl From<TextRange> for session::TextRange {
    /// Convert one bridge text range into one session text range.
    fn from(range: TextRange) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<TextEdit> for session::TextEdit {
    /// Convert one bridge text edit into one session text edit.
    fn from(edit: TextEdit) -> Self {
        Self {
            range: edit.range.into(),
            text: edit.text,
        }
    }
}

impl Display for SourceBridgeError {
    /// Format this source bridge error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Revision(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for SourceBridgeError {
    /// Return the underlying error source.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Revision(error) => Some(error),
        }
    }
}

impl From<RevisionParseError> for SourceBridgeError {
    /// Convert one revision parse error into one source bridge error.
    fn from(error: RevisionParseError) -> Self {
        Self::Revision(error)
    }
}
