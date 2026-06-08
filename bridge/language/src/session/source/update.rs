use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use destack_session as session;

use crate::{Revision, RevisionParseError, bridge};

use super::FileUpdate;

/// Source text range in byte offsets.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

/// Source text replacement.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Replaced byte range.
    pub range: TextRange,
    /// Replacement text.
    pub text: String,
}

/// One source edit accepted by a session update.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceEdit {
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
        /// Source repository logical path.
        from: String,
        /// Destination repository logical path.
        to: String,
    },
}

/// Source update applied through one session ref.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUpdate {
    /// Expected base revision.
    pub base: Option<Revision>,
    /// Source edits in this atomic update.
    pub edits: Vec<SourceEdit>,
}

/// Source update result.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUpdateResult {
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
    /// Changed files.
    pub files: Vec<FileUpdate>,
}

/// Error returned when a source bridge value cannot become a session value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceBridgeError {
    /// The revision id is invalid.
    Revision(crate::RevisionParseError),
}

impl SourceUpdateResult {
    /// Convert one session source update result through one live session.
    pub fn from_session_update(
        session: &session::Session,
        result: session::SourceUpdateResult,
    ) -> Self {
        let session::SourceUpdateResult {
            before,
            after,
            files,
        } = result;
        let files = files
            .into_iter()
            .map(|update| FileUpdate::from_session_update(session, update))
            .collect();

        Self {
            before: Revision::from_repository(before),
            after: Revision::from_repository(after),
            files,
        }
    }
}

impl TryFrom<SourceUpdate> for session::SourceUpdate {
    type Error = SourceBridgeError;

    /// Convert one bridge source update into one session source update.
    fn try_from(update: SourceUpdate) -> Result<Self, Self::Error> {
        let base = update
            .base
            .map(Revision::into_repository)
            .transpose()
            .map_err(SourceBridgeError::from)?;
        let edits = update
            .edits
            .into_iter()
            .map(session::SourceEdit::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(session::SourceUpdate { base, edits })
    }
}

impl TryFrom<SourceEdit> for session::SourceEdit {
    type Error = SourceBridgeError;

    /// Convert one bridge source edit into one session source edit.
    fn try_from(edit: SourceEdit) -> Result<Self, Self::Error> {
        match edit {
            SourceEdit::SetText { path, text } => Ok(Self::SetText {
                path: PathBuf::from(path),
                text,
            }),
            SourceEdit::EditText { path, edits } => Ok(Self::EditText {
                path: PathBuf::from(path),
                edits: edits.into_iter().map(session::TextEdit::from).collect(),
            }),
            SourceEdit::SetBytes { path, bytes } => Ok(Self::SetBytes {
                path: PathBuf::from(path),
                bytes,
            }),
            SourceEdit::Remove { path } => Ok(Self::Remove {
                path: PathBuf::from(path),
            }),
            SourceEdit::Move { from, to } => Ok(Self::Move {
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
