use std::collections::VecDeque;
use std::sync::Arc;

use im::OrdMap;
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use tspp_artifact::{
    ArtifactDependency, ArtifactEntry, ArtifactInvalidation, ArtifactKey, ArtifactTable,
    SourceDependency,
};

use crate::RepositoryError;

/// Artifact results selected by one repository revision.
#[derive(Debug, Clone, Default)]
pub(crate) struct ArtifactSelection {
    /// Artifact results inherited from preceding revisions.
    candidates: OrdMap<ArtifactKey, Arc<ArtifactEntry>>,
    /// Possibly changed dependency ordinals by artifact key.
    dirty: FxHashMap<ArtifactKey, SmallVec<[u32; 2]>>,
    /// Number of changes to the current artifact selection.
    generation: u64,
    /// Latest generation represented by the repository manifest.
    persisted_generation: u64,
}

impl ArtifactSelection {
    /// Build an empty artifact selection.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Return the current artifact selection generation.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Return whether the current selection requires persistent storage.
    pub(crate) fn needs_persistence(&self) -> bool {
        self.generation != self.persisted_generation
    }

    /// Return the number of artifact results proved current.
    pub(crate) fn len(&self) -> usize {
        self.candidates.len() - self.dirty.len()
    }

    /// Build one selection from exact completed artifact entries.
    pub(crate) fn from_entries(
        entries: impl IntoIterator<Item = Arc<ArtifactEntry>>,
    ) -> Result<Self, RepositoryError> {
        let mut selection = Self::new();
        for entry in entries {
            selection.select(entry)?;
        }
        selection.persisted_generation = selection.generation;

        Ok(selection)
    }

    /// Fork candidates and mark observations reached by changed sources.
    pub(crate) fn fork(&self, invalidated: &[SourceDependency], artifacts: &ArtifactTable) -> Self {
        let mut selection = self.clone();
        selection.generation += 1;
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
        self.generation += 1;

        Ok(())
    }

    /// Merge artifacts learned for an equal repository revision.
    pub(crate) fn adopt(&mut self, other: &Self) -> Result<(), RepositoryError> {
        if self.candidates.is_empty() && self.dirty.is_empty() {
            *self = other.clone();

            return Ok(());
        }

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

    /// Mark one successfully written generation as persistent.
    pub(crate) fn mark_persisted(&mut self, generation: u64) {
        if generation > self.persisted_generation {
            self.persisted_generation = generation;
        }
    }

    /// Return every artifact candidate retained by this revision.
    pub(crate) fn candidates(&self) -> Vec<Arc<ArtifactEntry>> {
        self.candidates.values().cloned().collect()
    }

    /// Iterate dependencies observed by every selected artifact candidate.
    pub(crate) fn dependencies(&self) -> impl Iterator<Item = &ArtifactDependency> {
        self.candidates
            .values()
            .flat_map(|entry| entry.dependencies.iter())
    }

    /// Return every artifact result proved current in this revision.
    pub(crate) fn current_entries(&self) -> Vec<Arc<ArtifactEntry>> {
        self.candidates
            .iter()
            .filter(|(key, _entry)| !self.dirty.contains_key(key))
            .map(|(_key, entry)| entry.clone())
            .collect()
    }
}
