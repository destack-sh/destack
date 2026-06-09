use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use destack_session as session;

use crate::{Revision, RevisionParseError, bridge};

use super::FileChange;

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

/// One file edit accepted by a session update.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEdit {
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

/// One file update applied through one session ref.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUpdate {
    /// Expected base revision.
    pub base: Option<Revision>,
    /// File edits in this atomic update.
    pub edits: Vec<FileEdit>,
}

/// File update result.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUpdateResult {
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
    /// Changed files.
    pub files: Vec<FileChange>,
}

/// Error returned when a source bridge value cannot become a session value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceBridgeError {
    /// The revision id is invalid.
    Revision(crate::RevisionParseError),
}

impl FileUpdateResult {
    /// Convert one session file update result through one live session.
    pub fn from_session_update(
        session: &session::Session,
        result: session::FileUpdateResult,
    ) -> Self {
        let session::FileUpdateResult {
            before,
            after,
            files,
        } = result;
        let files = files
            .into_iter()
            .map(|update| FileChange::from_session_update(session, update))
            .collect();

        Self {
            before: Revision::from_repository(before),
            after: Revision::from_repository(after),
            files,
        }
    }
}

impl TryFrom<FileUpdate> for session::FileUpdate {
    type Error = SourceBridgeError;

    /// Convert one bridge file update into one session file update.
    fn try_from(update: FileUpdate) -> Result<Self, Self::Error> {
        let base = update
            .base
            .map(Revision::into_repository)
            .transpose()
            .map_err(SourceBridgeError::from)?;
        let edits = update
            .edits
            .into_iter()
            .map(session::FileEdit::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(session::FileUpdate { base, edits })
    }
}

impl TryFrom<FileEdit> for session::FileEdit {
    type Error = SourceBridgeError;

    /// Convert one bridge file edit into one session file edit.
    fn try_from(edit: FileEdit) -> Result<Self, Self::Error> {
        match edit {
            FileEdit::SetText { path, text } => Ok(Self::SetText {
                path: PathBuf::from(path),
                text,
            }),
            FileEdit::EditText { path, edits } => Ok(Self::EditText {
                path: PathBuf::from(path),
                edits: edits.into_iter().map(session::TextEdit::from).collect(),
            }),
            FileEdit::SetBytes { path, bytes } => Ok(Self::SetBytes {
                path: PathBuf::from(path),
                bytes,
            }),
            FileEdit::Remove { path } => Ok(Self::Remove {
                path: PathBuf::from(path),
            }),
            FileEdit::Move { from, to } => Ok(Self::Move {
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
