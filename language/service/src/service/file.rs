use std::path::PathBuf;

use destack_session::{FileChangeKind, FileUpdate as SessionFileUpdate};
use destack_source::{Diagnostic, File, FileContent, FileId, FileType, ModuleId, Uri};

/// In-memory image for one updated file.
#[derive(Debug, Clone, PartialEq)]
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
        let content = match file.content.payload() {
            FileContent::Text { content } => Some(content.clone()),
            FileContent::Binary { .. } => None,
        };

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

/// File update emitted by the language service.
#[derive(Debug, Clone)]
pub struct FileUpdate {
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
    /// Updated file id.
    pub file_id: FileId,
    /// Diagnostic uri for this update.
    pub diagnostic_uri: Uri,
    /// Protocol file version for diagnostics when the file is open.
    pub diagnostic_version: Option<i32>,
    /// Updated file image.
    pub file: FileImage,
    /// The coarse change kind for this file.
    pub kind: FileChangeKind,
    /// Diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

impl From<SessionFileUpdate> for FileUpdate {
    /// Project one session file update into a service payload.
    fn from(update: SessionFileUpdate) -> Self {
        Self {
            module_id: update.module_id,
            file_id: update.file_id,
            diagnostic_uri: update.uri,
            diagnostic_version: None,
            file: FileImage::from(update.file.as_ref()),
            kind: update.kind,
            diagnostics: Vec::new(),
        }
    }
}
