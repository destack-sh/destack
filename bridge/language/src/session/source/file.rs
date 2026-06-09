use destack_session as session;

use crate::{ModuleId, bridge};

/// Observed file change projected from a file update.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// Repository logical path.
    pub path: String,
    /// External file URI.
    pub uri: String,
    /// Coarse file change kind.
    pub kind: FileChangeKind,
    /// Whether the file was removed.
    pub is_removed: bool,
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
}

/// One coarse kind for a file change.
#[bridge]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileChangeKind {
    /// One ordinary source change.
    Source,
    /// One `destack.json` change.
    Config,
}

impl FileChange {
    /// Convert one session file change through one live session.
    pub fn from_session_update(session: &session::Session, update: session::FileChange) -> Self {
        let path = session_file_change_path(session, &update);

        Self::from_session(path, update)
    }

    /// Convert one session file change into one bridge file change.
    pub fn from_session(path: String, update: session::FileChange) -> Self {
        let uri = update.uri().to_string();
        let kind = FileChangeKind::from_session(update.kind());
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

impl FileChangeKind {
    /// Convert one session file change kind into one bridge kind.
    pub fn from_session(kind: session::FileChangeKind) -> Self {
        match kind {
            session::FileChangeKind::Source => Self::Source,
            session::FileChangeKind::Config => Self::Config,
        }
    }

    /// Return the external file change kind label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Config => "config",
        }
    }
}

/// Return the repository logical path for one file change.
fn session_file_change_path(session: &session::Session, update: &session::FileChange) -> String {
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
