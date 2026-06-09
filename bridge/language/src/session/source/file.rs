use destack_session as session;

use crate::{ModuleId, bridge};

/// One file change observed by a session.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// Repository logical path.
    pub path: String,
    /// External file URI.
    pub uri: String,
    /// Whether the file was removed.
    pub is_removed: bool,
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
}

impl Change {
    /// Convert one session change through one live session.
    pub fn from_session_change(session: &session::Session, change: session::Change) -> Self {
        let path = session_change_path(session, &change);

        Self::from_session(path, change)
    }

    /// Convert one session change into one bridge change.
    pub fn from_session(path: String, change: session::Change) -> Self {
        let uri = change.uri().to_string();
        let is_removed = change.is_removed();
        let module_id = change.module_id().map(ModuleId::from_source);

        Self {
            path,
            uri,
            is_removed,
            module_id,
        }
    }
}

/// Return the repository logical path for one session change.
fn session_change_path(session: &session::Session, update: &session::Change) -> String {
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
