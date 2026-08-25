use std::collections::VecDeque;
use std::sync::Arc;

use destack_artifact::{
    ArtifactEntry, ArtifactInvalidation, ArtifactKey, ArtifactTable, SourceDependency,
};
use im::OrdMap;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use crate::RepositoryError;

/// Artifact results selected by one repository revision.
#[derive(Debug, Clone, Default)]
pub(crate) struct ArtifactSelection {
    /// Artifact results inherited from preceding revisions.
    candidates: OrdMap<ArtifactKey, Arc<ArtifactEntry>>,
    /// Possibly changed dependency ordinals by artifact key.
    dirty: FxHashMap<ArtifactKey, SmallVec<[u32; 2]>>,
}

impl ArtifactSelection {
    /// Build an empty artifact selection.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Fork candidates and mark observations reached by changed sources.
    pub(crate) fn fork(&self, invalidated: &[SourceDependency], artifacts: &ArtifactTable) -> Self {
        let mut selection = self.clone();
        let mut pending = invalidated
            .iter()
            .copied()
            .map(ArtifactInvalidation::Source)
            .collect::<VecDeque<_>>();
        let mut propagated = FxHashSet::default();

        // walk exact reverse edges from changed source observations
        while let Some(owner) = pending.pop_front() {
            for (dependent, dependencies) in artifacts.dependents(owner) {
                let key = dependent.version.key;
                let Some(candidate) = selection.candidates.get(&key) else {
                    continue;
                };
                if !Arc::ptr_eq(candidate, &dependent) {
                    continue;
                }

                let dirty = selection.dirty.entry(key).or_default();
                for dependency in dependencies {
                    if !dirty.contains(&dependency) {
                        dirty.push(dependency);
                    }
                }

                if propagated.insert(key) {
                    pending.push_back(ArtifactInvalidation::Artifact(dependent.version));
                    pending.push_back(ArtifactInvalidation::Projections(key));
                }
            }
        }

        for dependencies in selection.dirty.values_mut() {
            dependencies.sort_unstable();
        }

        selection
    }

    /// Return one artifact already proved current in this revision.
    pub(crate) fn current(&self, key: ArtifactKey) -> Option<Arc<ArtifactEntry>> {
        if self.dirty.contains_key(&key) {
            return None;
        }

        self.candidates.get(&key).cloned()
    }

    /// Return one inherited result available as an incremental base.
    pub(crate) fn base(&self, key: ArtifactKey) -> Option<Arc<ArtifactEntry>> {
        self.candidates.get(&key).cloned()
    }

    /// Return one candidate and its possibly changed dependency ordinals.
    pub(crate) fn candidate(
        &self,
        key: ArtifactKey,
    ) -> Option<(Arc<ArtifactEntry>, SmallVec<[u32; 2]>)> {
        let candidate = self.candidates.get(&key)?.clone();
        let dirty = self.dirty.get(&key).cloned().unwrap_or_default();

        Some((candidate, dirty))
    }

    /// Select one artifact result proved current in this revision.
    pub(crate) fn select(&mut self, entry: Arc<ArtifactEntry>) -> Result<(), RepositoryError> {
        let key = entry.version.key;
        if !self.dirty.contains_key(&key)
            && let Some(current) = self.candidates.get(&key)
        {
            if current.version != entry.version || current.dependencies != entry.dependencies {
                return Err(RepositoryError::InvalidArtifact {
                    message: format!(
                        "one revision selected conflicting artifact results: existing={:?}, new={:?}",
                        current.version, entry.version
                    ),
                });
            }

            return Ok(());
        }

        self.candidates.insert(key, entry);
        self.dirty.remove(&key);

        Ok(())
    }

    /// Merge artifacts learned for an equal repository revision.
    pub(crate) fn adopt(&mut self, other: &Self) -> Result<(), RepositoryError> {
        // select every result already proved current in the equal revision
        for (key, entry) in &other.candidates {
            if !other.dirty.contains_key(key) {
                self.select(entry.clone())?;
            }
        }

        // retain additional unresolved candidates and their sparse marks
        for (key, entry) in &other.candidates {
            if self.candidates.contains_key(key) {
                continue;
            }

            self.candidates.insert(*key, entry.clone());
            if let Some(dirty) = other.dirty.get(key) {
                self.dirty.insert(*key, dirty.clone());
            }
        }

        Ok(())
    }

    /// Return every artifact candidate retained by this revision.
    pub(crate) fn candidates(&self) -> Vec<Arc<ArtifactEntry>> {
        self.candidates.values().cloned().collect()
    }
}
