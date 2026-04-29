use std::fmt::{self, Display, Formatter};
use std::path::Path;
use std::sync::Arc;

use destack_core::stable_hash_value_256;
use im::OrdMap;
use serde::{Deserialize, Serialize};

use destack_source::{FileContentId, FileId};

use crate::repository::{FileEntry, HostEnvironment};

/// A ref names one movable repository tip.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ref(String);

impl Ref {
    /// Build a ref from one name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Build the canonical ref for one workspace root.
    pub fn for_workspace_root(root: &Path) -> Self {
        Self::new(format!("root:{}", root.display()))
    }

    /// Return the ref name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the ref into its name.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for Ref {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<String> for Ref {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Ref {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// The immutable handle for one published repository state.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub [u8; 32]);

impl Revision {
    /// The null revision handle.
    pub const NULL: Self = Self([0; 32]);

    /// Build one revision from one raw digest.
    pub const fn new(value: [u8; 32]) -> Self {
        Self(value)
    }

    /// Build one revision from one small test value.
    pub const fn from_test_value(value: u8) -> Self {
        Self([value; 32])
    }
}

impl fmt::Debug for Revision {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, formatter)
    }
}

impl Display for Revision {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("r")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }

        Ok(())
    }
}

/// The immutable state behind one revision.
#[derive(Debug, Clone)]
pub(crate) struct RevisionState {
    /// The files for this revision.
    pub files: Arc<OrdMap<FileId, FileEntry>>,
    /// The captured environment for this revision.
    pub host: Arc<HostEnvironment>,
}

impl RevisionState {
    /// Build one revision state record from explicit parts.
    pub(crate) fn new(files: Arc<OrdMap<FileId, FileEntry>>, host: Arc<HostEnvironment>) -> Self {
        Self { files, host }
    }

    /// Return the file content id for one file.
    pub(crate) fn file_content_id(&self, file_id: FileId) -> Option<FileContentId> {
        self.files.get(&file_id).map(|entry| entry.content_id)
    }

    /// Return the file entry for one file.
    pub(crate) fn file_entry(&self, file_id: FileId) -> Option<FileEntry> {
        self.files.get(&file_id).cloned()
    }

    /// Compute the deterministic identity for this revision state.
    pub(crate) fn revision(&self) -> Revision {
        Revision::new(stable_hash_value_256(&(&self.files, &self.host)))
    }
}
