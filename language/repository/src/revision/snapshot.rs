use std::hash::{Hash, Hasher};
use std::sync::Arc;

use im::OrdMap;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use destack_source::FileId;

use crate::repository::ContentId;

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
pub type SourceMap = OrdMap<FileId, ContentId>;

/// The immutable data behind one revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionData {
    /// The parent revisions for this snapshot.
    pub parents: SmallVec<[Revision; 2]>,
    /// The source map for this revision.
    pub source: Arc<SourceMap>,
}

impl RevisionData {
    /// Build one revision data record from explicit parts.
    pub fn new(parents: SmallVec<[Revision; 2]>, source: Arc<SourceMap>) -> Self {
        Self { parents, source }
    }

    /// Return the content id for one file.
    pub fn content_id(&self, file_id: FileId) -> Option<ContentId> {
        self.source.get(&file_id).copied()
    }

    /// Return whether one file exists in this revision.
    pub fn contains_file(&self, file_id: FileId) -> bool {
        self.source.contains_key(&file_id)
    }

    /// Compute the deterministic identity for this revision data.
    pub fn revision(&self) -> Revision {
        let mut hasher = FxHasher::default();

        // parents first
        self.parents.len().hash(&mut hasher);
        for parent in &self.parents {
            parent.hash(&mut hasher);
        }

        // canonical file map
        self.source.len().hash(&mut hasher);
        for (file_id, content_id) in self.source.iter() {
            file_id.hash(&mut hasher);
            content_id.hash(&mut hasher);
        }

        Revision::new(hasher.finish())
    }
}
