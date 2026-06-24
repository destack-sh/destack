use std::path::{Path, PathBuf};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// One byte range in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ByteRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

/// One source text replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TextPatch {
    /// Replaced byte range.
    pub range: ByteRange,
    /// Replacement text.
    pub text: String,
}

/// One source file mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Edit {
    /// Replace or create one text file.
    SetText {
        /// Repository relative path.
        path: PathBuf,
        /// Full text content.
        text: String,
    },
    /// Apply text replacements to one tracked text file.
    EditText {
        /// Repository relative path.
        path: PathBuf,
        /// Text replacements.
        patches: Vec<TextPatch>,
    },
    /// Replace or create one binary file.
    SetBytes {
        /// Repository relative path.
        path: PathBuf,
        /// Full binary content.
        bytes: Vec<u8>,
    },
    /// Remove one file.
    Remove {
        /// Repository relative path.
        path: PathBuf,
    },
    /// Move one file.
    Move {
        /// Source repository relative path.
        from: PathBuf,
        /// Destination repository relative path.
        to: PathBuf,
    },
}

impl Edit {
    /// Return the single file path affected by this edit.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::SetText { path, .. }
            | Self::EditText { path, .. }
            | Self::SetBytes { path, .. }
            | Self::Remove { path } => Some(path.as_path()),
            Self::Move { .. } => None,
        }
    }

    /// Return whether this edit removes its target file.
    pub fn is_remove(&self) -> bool {
        matches!(self, Self::Remove { .. })
    }

    /// Return full text content when this edit sets text directly.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::SetText { text, .. } => Some(text),
            _ => None,
        }
    }
}
