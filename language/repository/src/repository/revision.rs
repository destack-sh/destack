use std::fmt::{self, Display, Formatter};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_core::{TreapRoot, stable_hash_value_256};
use destack_source::FileId;
use parking_lot::{RwLock, RwLockWriteGuard};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::Environment;
use crate::repository::{Repository, RepositoryError, RevisionCache};

/// Content identity for one repository source and environment state.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub [u8; 32]);

impl Revision {
    /// The null revision identity.
    pub const NULL: Self = Self([0; 32]);

    /// Build one revision identity from one raw digest.
    pub const fn new(value: [u8; 32]) -> Self {
        Self(value)
    }

    /// Build one revision identity from one small test value.
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

/// A movable name pointing at one repository revision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ref(String);

impl Ref {
    /// Build a ref from one name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Build the canonical ref for one repository root.
    pub fn for_root(root: &Path) -> Self {
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

/// One retained repository revision.
#[derive(Debug)]
pub(crate) struct RevisionEntry {
    /// The retained revision state.
    state: Arc<RevisionState>,
    /// The active anonymous pin count.
    pin_count: AtomicUsize,
}

impl RevisionEntry {
    /// Build one revision entry from retained state.
    pub(crate) fn new(state: Arc<RevisionState>) -> Self {
        Self {
            state,
            pin_count: AtomicUsize::new(0),
        }
    }

    /// Return the retained revision state.
    pub(crate) fn state(&self) -> Arc<RevisionState> {
        Arc::clone(&self.state)
    }

    /// Increment the anonymous pin count.
    pub(crate) fn pin(&self) {
        self.pin_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the anonymous pin count.
    pub(crate) fn unpin(&self) {
        let _ = self
            .pin_count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count > 0).then_some(count - 1)
            });
    }

    /// Return whether this revision has active anonymous pins.
    pub(crate) fn is_pinned(&self) -> bool {
        self.pin_count.load(Ordering::Relaxed) > 0
    }
}

/// Source inputs, environment inputs, and predecessor bases for one revision.
#[derive(Debug)]
pub(crate) struct RevisionState {
    /// File bindings included in this revision.
    files: RwLock<TreapRoot>,
    /// Environment inputs captured in this revision.
    pub environment: Arc<Environment>,
    /// Base revisions considered when validating predecessor artifacts.
    bases: RwLock<Vec<RevisionBase>>,
    /// Lazily derived data for this revision.
    pub cache: RevisionCache,
}

impl RevisionState {
    /// Build one revision state from explicit parts.
    pub(crate) fn new(
        files: TreapRoot,
        environment: Arc<Environment>,
        bases: impl IntoIterator<Item = RevisionBase>,
    ) -> Self {
        Self {
            files: RwLock::new(files),
            environment,
            bases: RwLock::new(bases.into_iter().collect()),
            cache: RevisionCache::new(),
        }
    }

    /// Return the derived cache for this revision.
    pub(crate) fn cache(&self) -> &RevisionCache {
        &self.cache
    }

    /// Return the file binding root.
    pub(crate) fn files(&self) -> TreapRoot {
        *self.files.read()
    }

    /// Return this revision's base revisions.
    pub(crate) fn bases(&self) -> Vec<RevisionBase> {
        self.bases.read().clone()
    }

    /// Add base revisions to this revision.
    pub(crate) fn add_bases(&self, bases: impl IntoIterator<Item = RevisionBase>) {
        let mut stored = self.bases.write();

        // preserve nearest-first order while deduplicating branch joins
        for base in bases {
            if !stored.iter().any(|stored| stored.revision == base.revision) {
                stored.push(base);
            }
        }
    }

    /// Write the file binding root.
    pub(crate) fn write_files(&self) -> RwLockWriteGuard<'_, TreapRoot> {
        self.files.write()
    }

    /// Hash this revision state into its deterministic revision identity.
    pub(crate) fn revision(&self) -> Revision {
        Revision::new(stable_hash_value_256(&(&self.files(), &self.environment)))
    }
}

/// One predecessor edge into a revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevisionBase {
    /// The predecessor revision.
    pub(crate) revision: Revision,
    /// The source files changed between predecessor and child.
    pub(crate) delta: SourceDelta,
}

impl RevisionBase {
    /// Build one predecessor edge.
    pub(crate) fn new(revision: Revision, delta: SourceDelta) -> Self {
        Self { revision, delta }
    }
}

/// Source files changed by one revision edge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceDelta {
    /// The changed source file ids.
    files: Arc<[FileId]>,
}

impl SourceDelta {
    /// Build a source delta from changed files.
    pub(crate) fn new(files: impl IntoIterator<Item = FileId>) -> Self {
        let mut files = files.into_iter().collect::<Vec<_>>();
        files.sort_unstable();
        files.dedup();

        Self {
            files: files.into(),
        }
    }

    /// Return the changed source file ids.
    pub(crate) fn files(&self) -> &[FileId] {
        &self.files
    }

    /// Return whether this delta contains no changed source files.
    pub(crate) fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// One edge visited while searching a revision base path.
#[derive(Debug, Clone)]
struct DeltaPathStep {
    /// The revision reached by this path step.
    revision: Revision,
    /// The preceding step in the searched path.
    parent: Option<usize>,
    /// The source delta between this revision and its child path step.
    delta: SourceDelta,
}

impl Repository {
    /// Return source files changed between one ancestor and descendant revision.
    pub(crate) fn source_delta_between(
        &self,
        revision: Revision,
        ancestor: Revision,
    ) -> Result<SourceDelta, RepositoryError> {
        if revision == ancestor {
            return Ok(SourceDelta::default());
        }

        let mut steps = vec![DeltaPathStep {
            revision,
            parent: None,
            delta: SourceDelta::default(),
        }];
        let mut pending = vec![0];
        let mut seen = FxHashSet::default();
        let mut index = 0;

        // walk nearest base paths in deterministic order
        while index < pending.len() {
            let step = pending[index];
            let revision = steps[step].revision;
            index += 1;

            if !seen.insert(revision) {
                continue;
            }

            let revision = self.revision(revision)?;
            for base in revision.bases() {
                let next = steps.len();
                steps.push(DeltaPathStep {
                    revision: base.revision,
                    parent: Some(step),
                    delta: base.delta,
                });

                if base.revision == ancestor {
                    return Ok(source_delta_from_path(&steps, next));
                }

                pending.push(next);
            }
        }

        Err(RepositoryError::UnrelatedRevision { revision, ancestor })
    }

    /// Return ancestors of one revision in nearest-first deterministic order.
    pub(crate) fn revision_ancestors(
        &self,
        revision: Revision,
    ) -> Result<Vec<Revision>, RepositoryError> {
        let revision = self.revision(revision)?;
        let mut ancestors = Vec::new();
        let mut pending = revision.bases();
        let mut seen = FxHashSet::default();
        let mut index = 0;

        // walk base graph breadth first
        while index < pending.len() {
            let ancestor = pending[index].revision;
            index += 1;

            // skip already visited bases
            if !seen.insert(ancestor) {
                continue;
            }

            // skip pruned bases
            let Some(ancestor_state) = self.revisions.get(&ancestor) else {
                continue;
            };

            let ancestor_state = ancestor_state.state();
            ancestors.push(ancestor);
            pending.extend(ancestor_state.bases());
        }

        Ok(ancestors)
    }
}

/// Build the source delta represented by one discovered base path.
fn source_delta_from_path(steps: &[DeltaPathStep], mut step: usize) -> SourceDelta {
    let mut files = Vec::new();

    // walk back to the descendant and copy source file ids once
    loop {
        let path_step = &steps[step];
        if !path_step.delta.is_empty() {
            files.extend(path_step.delta.files().iter().copied());
        }

        let Some(parent) = path_step.parent else {
            break;
        };
        step = parent;
    }

    SourceDelta::new(files)
}
