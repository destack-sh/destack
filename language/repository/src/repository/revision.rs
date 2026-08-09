use std::fmt::{self, Display, Formatter};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_artifact::SourceDependency;
use destack_core::{TreapRoot, stable_hash_value_256};
use destack_serde::Reflect;
use parking_lot::{RwLock, RwLockWriteGuard};
use serde::{Deserialize, Serialize};

use crate::repository::{Repository, RepositoryError, RevisionCache};
use crate::{ArtifactBindingTable, Environment};

/// Content identity for one repository source and environment state.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
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
        self.state.clone()
    }

    /// Increment the anonymous pin count.
    pub(crate) fn pin(&self) {
        self.pin_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement the anonymous pin count.
    pub(crate) fn unpin(&self) {
        let result = self
            .pin_count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                (count > 0).then_some(count - 1)
            });
        if result.is_err() {
            unreachable!("cannot release an unpinned repository revision");
        }
    }

    /// Return whether this revision has active anonymous pins.
    pub(crate) fn is_pinned(&self) -> bool {
        self.pin_count.load(Ordering::Relaxed) > 0
    }
}

/// Source state and derived artifact bindings for one revision.
#[derive(Debug)]
pub(crate) struct RevisionState {
    /// File bindings included in this revision.
    files: RwLock<TreapRoot>,
    /// Environment inputs captured in this revision.
    pub environment: Arc<Environment>,
    /// Derived artifact bindings.
    pub(crate) artifacts: ArtifactBindingTable,
    /// Lazily derived data for this revision.
    pub cache: RevisionCache,
}

impl RevisionState {
    /// Build one revision state from explicit parts.
    pub(crate) fn new(
        files: TreapRoot,
        environment: Arc<Environment>,
        artifacts: ArtifactBindingTable,
    ) -> Self {
        Self {
            files: RwLock::new(files),
            environment,
            artifacts,
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

    /// Write the file binding root.
    pub(crate) fn write_files(&self) -> RwLockWriteGuard<'_, TreapRoot> {
        self.files.write()
    }

    /// Hash this revision state into its deterministic revision identity.
    pub(crate) fn revision(&self) -> Revision {
        Revision::new(stable_hash_value_256(&(&self.files(), &self.environment)))
    }
}

/// Source changes applied while forking one revision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceDelta {
    /// The exact source observations invalidated by the edits.
    sources: Arc<[SourceDependency]>,
    /// Whether package or module discovery inputs changed.
    is_discovery_changed: bool,
    /// Whether a package config file changed.
    is_config_changed: bool,
}

impl SourceDelta {
    /// Build a source delta from invalidated observations.
    pub(crate) fn new(
        sources: impl IntoIterator<Item = SourceDependency>,
        is_discovery_changed: bool,
        is_config_changed: bool,
    ) -> Self {
        let mut sources = sources.into_iter().collect::<Vec<_>>();
        sources.sort_unstable();
        sources.dedup();

        Self {
            sources: sources.into(),
            is_discovery_changed,
            is_config_changed,
        }
    }

    /// Add invalidated source observations.
    pub(crate) fn extend(&mut self, invalidated: impl IntoIterator<Item = SourceDependency>) {
        let mut sources = self.sources.to_vec();
        sources.extend(invalidated);
        sources.sort_unstable();
        sources.dedup();
        self.sources = sources.into();
    }

    /// Return the invalidated source observations.
    pub(crate) fn sources(&self) -> &[SourceDependency] {
        &self.sources
    }

    /// Return whether package or module discovery inputs changed.
    pub(crate) fn is_discovery_changed(&self) -> bool {
        self.is_discovery_changed
    }

    /// Return whether a package config file changed.
    pub(crate) fn is_config_changed(&self) -> bool {
        self.is_config_changed
    }
}

impl Repository {
    /// Return the ambient environment captured by one revision.
    pub fn environment(&self, revision: Revision) -> Result<Arc<Environment>, RepositoryError> {
        let revision = self.revision(revision)?;

        Ok(revision.environment.clone())
    }
}
