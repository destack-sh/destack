use std::collections::VecDeque;

use destack_artifact::{ArtifactBindingId, ArtifactDependencyOwner, ArtifactId, ArtifactTable};
use parking_lot::{RwLock, RwLockReadGuard};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;

use crate::{Delta, RepositoryError};

/// Artifact bindings retained by one repository revision.
#[derive(Debug, Default)]
pub(crate) struct ArtifactBindingTable {
    /// Persistent bindings and sparse invalidation state.
    bindings: RwLock<ArtifactBindings>,
}

impl ArtifactBindingTable {
    /// Build an empty artifact binding table.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Fork these bindings and dirty artifacts reachable from one source delta.
    pub(crate) fn fork(
        &self,
        delta: &Delta,
        artifacts: &ArtifactTable,
    ) -> Result<Self, RepositoryError> {
        let mut bindings = self.bindings.read().clone();
        let mut pending = delta
            .invalidated()
            .iter()
            .copied()
            .map(ArtifactDependencyOwner::Source)
            .collect::<VecDeque<_>>();

        // collect exact dirty dependency ordinals through direct reverse edges
        let mut changes = FxHashMap::<ArtifactId, SmallVec<[u32; 2]>>::default();
        let mut propagated = FxHashSet::default();
        while let Some(owner) = pending.pop_front() {
            let dependents = artifacts.dependents(owner);

            for dependent in dependents {
                let Some(binding_id) = bindings.binding(dependent.artifact) else {
                    continue;
                };
                if binding_id != dependent.binding {
                    continue;
                }
                let dependencies = changes.entry(dependent.artifact).or_default();
                if !dependencies.contains(&dependent.dependency) {
                    dependencies.push(dependent.dependency);
                }

                if propagated.insert(dependent.artifact) {
                    let binding = artifacts.binding(binding_id).ok_or(
                        RepositoryError::MissingArtifactBindingId {
                            binding: binding_id,
                        },
                    )?;
                    pending.push_back(ArtifactDependencyOwner::Artifact(binding.version));
                    pending.push_back(ArtifactDependencyOwner::Projection(binding.version.key));
                }
            }
        }

        // merge each affected artifact's sparse dependency changes
        for (artifact, dependencies) in changes {
            bindings.mark_dirty(artifact, dependencies);
        }

        Ok(Self {
            bindings: RwLock::new(bindings),
        })
    }

    /// Return one artifact binding state.
    pub(crate) fn state(&self, artifact: ArtifactId) -> Option<ArtifactBindingState> {
        self.bindings.read().state(artifact)
    }

    /// Return a read view shared across one resolution batch.
    pub(crate) fn snapshot(&self) -> RwLockReadGuard<'_, ArtifactBindings> {
        self.bindings.read()
    }

    /// Return every retained artifact binding id.
    pub(crate) fn bindings(&self) -> Vec<ArtifactBindingId> {
        self.bindings
            .read()
            .values
            .iter()
            .flatten()
            .copied()
            .collect()
    }

    /// Bind one current result.
    pub(crate) fn bind(&self, artifact: ArtifactId, binding: ArtifactBindingId) {
        self.bindings.write().bind(artifact, binding);
    }

    /// Adopt bindings this table lacks from one equal content state.
    pub(crate) fn adopt(&self, other: &Self) {
        let other = other.bindings.read();
        let mut bindings = self.bindings.write();
        if bindings.values.len() < other.values.len() {
            bindings.values.resize(other.values.len(), None);
        }

        // fill absent bindings and carry their dirty ordinals along
        for (index, binding) in other.values.iter().enumerate() {
            let Some(binding) = binding else {
                continue;
            };
            if bindings.values[index].is_none() {
                bindings.values[index] = Some(*binding);
                let artifact = ArtifactId::from_index(index);
                if let Some(dirty) = other.dirty.get(&artifact) {
                    bindings.dirty.insert(artifact, dirty.clone());
                }
            }
        }
    }

    /// Commit resolved bindings when every observed selection still matches.
    pub(crate) fn commit(
        &self,
        observations: &FxHashMap<ArtifactId, Option<ArtifactBindingState>>,
        bindings: &[(ArtifactId, ArtifactBindingId)],
    ) -> Result<bool, RepositoryError> {
        let mut current = self.bindings.write();

        // reject only artifacts that changed during this resolution
        for (artifact, observed) in observations {
            if current.state(*artifact) != *observed {
                return Ok(false);
            }
        }

        // publish refreshed and reused bindings, clearing dependencies proven unchanged
        for (artifact, binding) in bindings {
            let was_dirty = current.dirty.remove(artifact).is_some();
            let was_absent = matches!(observations.get(artifact), Some(None));
            if !was_dirty && !was_absent {
                return Err(RepositoryError::InvalidArtifact {
                    message: format!("cannot refresh clean or missing artifact {artifact:?}"),
                });
            }
            current.set(*artifact, *binding);
        }

        Ok(true)
    }
}

/// Persistent bindings and sparse dirty dependency ordinals.
#[derive(Debug, Clone, Default)]
pub(crate) struct ArtifactBindings {
    /// Immutable bindings by artifact.
    values: Vec<Option<ArtifactBindingId>>,
    /// Dirty dependency ordinals for affected artifacts only.
    dirty: FxHashMap<ArtifactId, SmallVec<[u32; 2]>>,
}

impl ArtifactBindings {
    /// Return one artifact binding state.
    pub(crate) fn state(&self, artifact: ArtifactId) -> Option<ArtifactBindingState> {
        let binding = self.binding(artifact)?;
        let dirty_dependencies = self.dirty.get(&artifact).cloned().unwrap_or_default();

        Some(ArtifactBindingState {
            binding,
            dirty_dependencies,
        })
    }

    /// Return one selected artifact binding id.
    fn binding(&self, artifact: ArtifactId) -> Option<ArtifactBindingId> {
        self.values.get(artifact.index()).copied().flatten()
    }

    /// Bind one current result and clear its dirty dependencies.
    fn bind(&mut self, artifact: ArtifactId, binding: ArtifactBindingId) {
        self.set(artifact, binding);
        self.dirty.remove(&artifact);
    }

    /// Select one artifact binding.
    fn set(&mut self, artifact: ArtifactId, binding: ArtifactBindingId) {
        let index = artifact.index();
        if self.values.len() <= index {
            self.values.resize(index + 1, None);
        }
        self.values[index] = Some(binding);
    }

    /// Merge changed dependency ordinals into one artifact.
    fn mark_dirty(&mut self, artifact: ArtifactId, dependencies: SmallVec<[u32; 2]>) {
        let mut dirty = match self.dirty.get(&artifact) {
            Some(dependencies) => dependencies.clone(),
            None => SmallVec::new(),
        };
        for dependency in dependencies {
            if !dirty.contains(&dependency) {
                dirty.push(dependency);
            }
        }
        dirty.sort_unstable();
        self.dirty.insert(artifact, dirty);
    }
}

/// One artifact binding and its possibly changed dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactBindingState {
    /// The immutable repository-wide binding.
    pub(crate) binding: ArtifactBindingId,
    /// Dependency ordinals that may have changed.
    pub(crate) dirty_dependencies: SmallVec<[u32; 2]>,
}

impl ArtifactBindingState {
    /// Return whether this artifact has no dirty dependencies.
    pub(crate) fn is_clean(&self) -> bool {
        self.dirty_dependencies.is_empty()
    }
}
