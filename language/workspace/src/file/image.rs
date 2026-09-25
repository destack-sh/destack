use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::{File, FileId, FileType, Uri};

use crate::Error;

/// In-memory image for one updated file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileImage {
    /// File id in the registry.
    pub id: FileId,
    /// File name.
    pub name: String,
    /// File uri.
    pub uri: Uri,
    /// Optional file path.
    pub path: Option<PathBuf>,
    /// File type.
    pub file_type: FileType,
    /// Optional text content.
    pub content: Option<String>,
}

impl From<&File> for FileImage {
    /// Build a file image from one source file.
    fn from(file: &File) -> Self {
        let content = file.is_text().then(|| file.text().to_string());

        Self {
            id: file.id,
            name: file.name.clone(),
            uri: file.uri.clone(),
            path: file.path.clone(),
            file_type: file.ty,
            content,
        }
    }
}

impl FileImage {
    /// Convert this image into a source file.
    pub fn into_file(self) -> Result<File, Error> {
        let Some(content) = self.content else {
            return Err(Error::Internal {
                detail: format!("file image is missing text content for {}", self.name),
            });
        };

        File::from_text(
            self.id,
            self.name,
            self.uri,
            self.path,
            self.file_type,
            content,
        )
        .map_err(|error| Error::Internal {
            detail: error.to_string(),
        })
    }
}
