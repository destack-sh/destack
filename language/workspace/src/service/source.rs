use std::path::PathBuf;
use std::sync::Arc;

use destack_repository::Revision;
use destack_serde::Reflect;
use destack_source::{FileId, Patch, TextRange};
use serde::{Deserialize, Serialize};

use crate::{Error, FileEdit, FileImage, FileOperation, SourceUpdate};

/// Request to apply one workspace file operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ApplyFileOperationRequest {
    /// File operation to apply.
    pub operation: FileOperation,
}

/// Request to apply one atomic source update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ApplySourceUpdateRequest {
    /// Root receiving the source update.
    pub root: PathBuf,
    /// Source update to apply.
    pub update: SourceUpdate,
}

/// Request to inspect whether one file is open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct IsFileOpenRequest {
    /// Source path to inspect.
    pub path: PathBuf,
}

/// Request to format one source file or selected range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FormatFileRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Source path to format.
    pub path: PathBuf,
    /// Optional UTF-16 source range.
    pub range: Option<TextRange>,
}

/// Request to read source files at one exact revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadFilesRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact semantic revision.
    pub revision: Revision,
    /// Source file identifiers.
    pub file_ids: Vec<FileId>,
}

/// Serialized source edit over one exact file image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileEditResponse {
    /// Exact source file image interpreted by the patch.
    pub file: FileImage,
    /// Source replacement.
    pub patch: Patch,
}

impl TryFrom<FileEditResponse> for FileEdit {
    type Error = Error;

    /// Convert one serialized response into a runtime file edit.
    fn try_from(response: FileEditResponse) -> Result<Self, Self::Error> {
        let file = response.file.into_file()?;

        Ok(Self {
            file: Arc::new(file),
            patch: response.patch,
        })
    }
}

impl From<&FileEdit> for FileEditResponse {
    /// Build one serialized response from a runtime file edit.
    fn from(edit: &FileEdit) -> Self {
        Self {
            file: FileImage::from(edit.file.as_ref()),
            patch: edit.patch.clone(),
        }
    }
}
