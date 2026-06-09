// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{FileId, Span};

/// One source edit crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct Edit {
    /// Source span to replace.
    pub span: Span,
    /// Replacement text.
    pub new_text: String,
}

impl Edit {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::Edit) -> Self {
        Self {
            span: Span::from_bridge(value.span),
            new_text: value.new_text,
        }
    }
}

/// Edits for a single file.
#[derive(Debug)]
#[napi(object)]
pub struct FilePatch {
    /// Edited file.
    pub file: FileId,
    /// Source edits.
    pub edits: Vec<Edit>,
}

impl FilePatch {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::FilePatch) -> Self {
        Self {
            file: FileId::from_bridge(value.file),
            edits: value
                .edits
                .into_iter()
                .map(|item| Edit::from_bridge(item))
                .collect(),
        }
    }
}

/// Edits across multiple files.
#[derive(Debug)]
#[napi(object)]
pub struct BatchEdit {
    /// Per-file edits.
    pub files: Vec<FilePatch>,
}

impl BatchEdit {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::BatchEdit) -> Self {
        Self {
            files: value
                .files
                .into_iter()
                .map(|item| FilePatch::from_bridge(item))
                .collect(),
        }
    }
}
