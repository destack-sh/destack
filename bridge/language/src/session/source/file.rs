use destack_session as session;

use crate::{ModuleId, bridge};

/// One source file in an explicit source snapshot.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Repository-root relative path.
    pub path: String,
    /// Full source file content.
    pub content: SourceFileContent,
}

/// Full content for one source snapshot file.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFileContent {
    /// Text file content.
    Text(String),
    /// Binary file content.
    Bytes(Vec<u8>),
}

/// File update projected from a source update.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUpdate {
    /// Repository logical path.
    pub path: String,
    /// External file URI.
    pub uri: String,
    /// Coarse file update kind.
    pub kind: FileUpdateKind,
    /// Whether the file was removed.
    pub is_removed: bool,
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
}

/// One coarse kind for a file change.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileUpdateKind {
    /// One ordinary source change.
    Source,
    /// One `destack.json` change.
    Config,
}

impl SourceFile {
    /// Create one text source file.
    pub fn text(path: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: SourceFileContent::Text(text.into()),
        }
    }

    /// Create one binary source file.
    pub fn bytes(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.into(),
            content: SourceFileContent::Bytes(bytes.into()),
        }
    }
}

impl FileUpdate {
    /// Convert one session file update through one live session.
    pub fn from_session_update(session: &session::Session, update: session::FileUpdate) -> Self {
        let path = session_file_update_path(session, &update);

        Self::from_session(path, update)
    }

    /// Convert one session file update into one bridge file update.
    pub fn from_session(path: String, update: session::FileUpdate) -> Self {
        let uri = update.uri().to_string();
        let kind = FileUpdateKind::from_session(update.kind());
        let is_removed = update.is_removed();
        let module_id = update.module_id().map(ModuleId::from_source);

        Self {
            path,
            uri,
            kind,
            is_removed,
            module_id,
        }
    }
}

impl FileUpdateKind {
    /// Convert one session file update kind into one bridge kind.
    pub fn from_session(kind: session::FileUpdateKind) -> Self {
        match kind {
            session::FileUpdateKind::Source => Self::Source,
            session::FileUpdateKind::Config => Self::Config,
        }
    }

    /// Return the external file update kind label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Config => "config",
        }
    }
}

impl TryFrom<SourceFile> for session::SourceFile {
    type Error = super::SourceBridgeError;

    /// Convert one bridge source file into one session source file.
    fn try_from(file: SourceFile) -> Result<Self, Self::Error> {
        match file.content {
            SourceFileContent::Text(text) => Ok(session::SourceFile::text(file.path, text)),
            SourceFileContent::Bytes(bytes) => Ok(session::SourceFile::bytes(file.path, bytes)),
        }
    }
}

/// Return the repository logical path for one file update.
fn session_file_update_path(session: &session::Session, update: &session::FileUpdate) -> String {
    if let Some(file) = update.file()
        && let Some(path) = file.path.as_deref()
    {
        return session.repository_path(path);
    }

    if let Some(path) = update.uri().to_path_buf() {
        return session.repository_path(&path);
    }

    update.uri().to_string()
}
