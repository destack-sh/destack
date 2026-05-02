use std::path::PathBuf;

use destack_session::{FileUpdate as SessionFileUpdate, FileUpdateKind};
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
    /// Updated file image when the file still exists.
    pub file: Option<FileImage>,
    /// Whether this update removed the file.
    pub is_removed: bool,
    /// The coarse change kind for this file.
    pub kind: FileUpdateKind,
    /// Diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

impl From<SessionFileUpdate> for FileUpdate {
    /// Project one session file update into a service payload.
    fn from(update: SessionFileUpdate) -> Self {
        match update {
            SessionFileUpdate::Updated {
                module_id,
                file_id,
                uri,
                file,
                kind,
            } => Self {
                module_id,
                file_id,
                diagnostic_uri: uri,
                diagnostic_version: None,
                file: Some(FileImage::from(file.as_ref())),
                is_removed: false,
                kind,
                diagnostics: Vec::new(),
            },
            SessionFileUpdate::Removed {
                module_id,
                file_id,
                uri,
                kind,
            } => Self {
                module_id,
                file_id,
                diagnostic_uri: uri,
                diagnostic_version: None,
                file: None,
                is_removed: true,
                kind,
                diagnostics: Vec::new(),
            },
        }
    }
}
