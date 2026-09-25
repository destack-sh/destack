use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_repository::Revision;
use tspp_serde::Reflect;
use tspp_source::{Edit, FileId, Patch, TextRange};

use crate::{Error, FileEdit, FileImage, FileSelection};

/// Request to edit physical workspace state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct EditRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact expected physical revision.
    pub revision: Revision,
    /// Source file edits.
    pub edits: Vec<Edit>,
}

/// Request to edit one workspace branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct EditBranchRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Branch receiving the source edits.
    pub name: String,
    /// Exact expected branch revision.
    pub revision: Revision,
    /// Source file edits.
    pub edits: Vec<Edit>,
}

/// Request to save one branch to the physical workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SaveBranchRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Branch to save.
    pub name: String,
    /// Exact branch revision to save.
    pub revision: Revision,
    /// Expected physical workspace revision.
    pub physical: Revision,
    /// Files to save.
    pub files: FileSelection,
}

/// Request to restore one branch from physical workspace state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RestoreBranchRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Branch receiving restored files.
    pub name: String,
    /// Expected branch revision.
    pub revision: Revision,
    /// Expected physical workspace revision.
    pub physical: Revision,
    /// Files to restore.
    pub files: FileSelection,
}

/// Request to compare two exact workspace revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DiffRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
}

/// Request to list files at one exact workspace revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ListFilesRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact source revision.
    pub revision: Revision,
}

/// Request to format one source file or selected range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FormatFileRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact source revision.
    pub revision: Revision,
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
