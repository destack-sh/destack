use destack_session as session;

use crate::bridge;

use super::{SourceBridgeError, SourceFile};

/// Source truth used to open a live session.
#[bridge]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceSnapshot {
    /// Files visible to the session source root.
    pub files: Vec<SourceFile>,
}

impl SourceSnapshot {
    /// Create one source snapshot from explicit files.
    pub fn new(files: Vec<SourceFile>) -> Self {
        Self { files }
    }
}

impl TryFrom<SourceSnapshot> for session::SourceSnapshot {
    type Error = SourceBridgeError;

    /// Convert one bridge source snapshot into one session source snapshot.
    fn try_from(snapshot: SourceSnapshot) -> Result<Self, Self::Error> {
        let files = snapshot
            .files
            .into_iter()
            .map(session::SourceFile::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(session::SourceSnapshot::new(files))
    }
}
