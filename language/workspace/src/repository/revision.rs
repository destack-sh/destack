use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use im::OrdMap;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use destack_source::FileId;

use crate::repository::FileContentId;

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

impl std::fmt::Display for Ref {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// The immutable handle for one published source snapshot.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub u64);

impl Revision {
    /// The empty root revision handle.
    pub const INITIAL: Self = Self(0);

    /// Build one revision from one raw hash value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for Revision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "r{:016x}", self.0)
    }
}

/// The structurally shared source map for one revision.
pub type SourceMap = OrdMap<FileId, FileContentId>;

/// The immutable state behind one revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionState {
    /// The parent revisions for this snapshot.
    pub parents: SmallVec<[Revision; 2]>,
    /// The source map for this revision.
    pub source: Arc<SourceMap>,
}

impl RevisionState {
    /// Build one revision state record from explicit parts.
    pub fn new(parents: SmallVec<[Revision; 2]>, source: Arc<SourceMap>) -> Self {
        Self { parents, source }
    }

    /// Return the file content id for one file.
    pub fn file_content_id(&self, file_id: FileId) -> Option<FileContentId> {
        self.source.get(&file_id).copied()
    }

    /// Return whether one file exists in this revision.
    pub fn contains_file(&self, file_id: FileId) -> bool {
        self.source.contains_key(&file_id)
    }

    /// Compute the deterministic identity for this revision state.
    pub fn revision(&self) -> Revision {
        let mut hasher = FxHasher::default();

        // canonical file map
        self.source.len().hash(&mut hasher);
        for (file_id, content_id) in self.source.iter() {
            file_id.hash(&mut hasher);
            content_id.hash(&mut hasher);
        }

        Revision::new(hasher.finish())
    }
}
