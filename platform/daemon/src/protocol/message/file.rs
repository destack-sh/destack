use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use destack_source::{FileId, FileType, Uri};

/// File snapshot for protocol responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSnapshot {
    /// The file id.
    pub id: FileId,
    /// The file name.
    pub name: String,
    /// The file uri.
    pub uri: Uri,
    /// Optional file path.
    pub path: Option<PathBuf>,
    /// The file type.
    pub file_type: FileType,
    /// Optional file content.
    pub content: Option<String>,
}
