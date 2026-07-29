use std::collections::VecDeque;

use destack_artifact::{
    ArtifactBindingId, ArtifactDependency, ArtifactId, ArtifactKey, SourceDependency,
};
use destack_source::FileId;
use im::{HashMap, HashSet};
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use smallvec::SmallVec;

use crate::{RepositoryError, SourceDelta};

/// Direct reverse dependency edges for one dependency owner.
type ArtifactDependents = HashSet<ArtifactDependent, FxBuildHasher>;

/// One revision's persistent artifact dependency graph.
#[derive(Debug, Clone, Default)]
pub(crate) struct ArtifactGraph {
    /// The revision-local mutation generation.
    generation: u64,
    /// Current immutable binding selections.
    bindings: HashMap<ArtifactId, ArtifactBindingId, FxBuildHasher>,
    /// Dirty dependency ordinals for affected artifacts only.
    dirty: HashMap<ArtifactId, SmallVec<[u32; 2]>, FxBuildHasher>,
    /// Direct reverse dependency edges.
    dependents: HashMap<ArtifactDependencyOwner, ArtifactDependents, FxBuildHasher>,
}

impl ArtifactGraph {
    /// Build an empty artifact graph.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Fork this graph and dirty artifacts reachable from one source delta.
    pub(crate) fn fork(&self, delta: &SourceDelta) -> Result<Self, RepositoryError> {
        let mut graph = self.clone();
        graph.generation = 0;
        let mut pending = delta
            .files()
            .iter()
            .copied()
            .map(ArtifactDependencyOwner::File)
            .collect::<VecDeque<_>>();

        // invalidate repository membership observations
        if delta.is_discovery_changed() {
            pending.push_back(ArtifactDependencyOwner::Packages);
            pending.push_back(ArtifactDependencyOwner::Modules);
        }

        // collect exact dirty dependency ordinals through direct reverse edges
        let mut changes = FxHashMap::<ArtifactId, SmallVec<[u32; 2]>>::default();
        let mut propagated = FxHashSet::default();
        while let Some(owner) = pending.pop_front() {
            let Some(dependents) = graph.dependents.get(&owner) else {
                continue;
            };

            for dependent in dependents {
                let dependencies = changes.entry(dependent.artifact).or_default();
                if !dependencies.contains(&dependent.dependency) {
                    dependencies.push(dependent.dependency);
                }

                if propagated.insert(dependent.artifact) {
                    pending.push_back(ArtifactDependencyOwner::Artifact(dependent.artifact));
                }
            }
        }

        // merge each affected artifact's sparse dependency changes
        for (artifact, dependencies) in changes {
            if !graph.bindings.contains_key(&artifact) {
                return Err(RepositoryError::InvalidArtifactGraph {
                    message: format!("reverse dependency references missing artifact {artifact:?}"),
                });
            }
            let mut dirty = graph.dirty.get(&artifact).cloned().unwrap_or_default();
            for dependency in dependencies {
                if !dirty.contains(&dependency) {
                    dirty.push(dependency);
                }
            }
            dirty.sort_unstable();
            graph.dirty.insert(artifact, dirty);
        }

        Ok(graph)
    }

    /// Return one artifact state.
    pub(crate) fn state(&self, artifact: ArtifactId) -> Option<ArtifactState> {
        let binding = self.binding(artifact)?;
        let dirty_dependencies = self.dirty.get(&artifact).cloned().unwrap_or_default();

        Some(ArtifactState {
            binding,
            dirty_dependencies,
        })
    }

    /// Return one selected artifact binding id.
    pub(crate) fn binding(&self, artifact: ArtifactId) -> Option<ArtifactBindingId> {
        self.bindings.get(&artifact).copied()
    }

    /// Return the revision-local mutation generation.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Return every selected artifact binding id.
    pub(crate) fn bindings(&self) -> impl Iterator<Item = ArtifactBindingId> + '_ {
        self.bindings.values().copied()
    }

    /// Bind one current result and replace its reverse dependency edges.
    pub(crate) fn bind(
        &mut self,
        artifact: ArtifactId,
        binding: ArtifactBindingId,
        previous: Option<&[ArtifactDependency]>,
        dependencies: &[ArtifactDependency],
        artifact_id: impl Fn(ArtifactKey) -> Option<ArtifactId>,
    ) -> Result<(), RepositoryError> {
        // update only reverse edges whose owner or ordinal changed
        let previous = previous.unwrap_or_default();
        let dependency_count = previous.len().max(dependencies.len());
        for dependency in 0..dependency_count {
            let dependent = ArtifactDependent {
                artifact,
                dependency: dependency as u32,
            };
            let previous_owner = previous
                .get(dependency)
                .map(|dependency| ArtifactDependencyOwner::new(dependency, &artifact_id))
                .transpose()?;
            let owner = dependencies
                .get(dependency)
                .map(|dependency| ArtifactDependencyOwner::new(dependency, &artifact_id))
                .transpose()?;
            if previous_owner == owner {
                continue;
            }

            if let Some(previous_owner) = previous_owner {
                self.remove_dependent(previous_owner, dependent)?;
            }
            if let Some(owner) = owner {
                self.insert_dependent(owner, dependent);
            }
        }

        // select the new current binding
        let previous_binding = self.bindings.insert(artifact, binding);
        let dirty = self.dirty.remove(&artifact);
        if previous_binding != Some(binding) || dirty.is_some() {
            self.generation += 1;
        }

        Ok(())
    }

    /// Clear one artifact's resolved dirty dependencies.
    pub(crate) fn refresh(&mut self, artifact: ArtifactId) -> Result<(), RepositoryError> {
        if self.dirty.remove(&artifact).is_none() {
            return Err(RepositoryError::InvalidArtifactGraph {
                message: format!("cannot refresh clean or missing artifact {artifact:?}"),
            });
        }
        self.generation += 1;

        Ok(())
    }

    /// Insert one direct reverse dependency edge.
    fn insert_dependent(&mut self, owner: ArtifactDependencyOwner, dependent: ArtifactDependent) {
        let mut dependents = self.dependents.get(&owner).cloned().unwrap_or_default();
        dependents.insert(dependent);
        self.dependents.insert(owner, dependents);
    }

    /// Remove one direct reverse dependency edge.
    fn remove_dependent(
        &mut self,
        owner: ArtifactDependencyOwner,
        dependent: ArtifactDependent,
    ) -> Result<(), RepositoryError> {
        let Some(mut dependents) = self.dependents.get(&owner).cloned() else {
            return Err(RepositoryError::InvalidArtifactGraph {
                message: format!("missing reverse dependency owner {owner:?}"),
            });
        };
        if dependents.remove(&dependent).is_none() {
            return Err(RepositoryError::InvalidArtifactGraph {
                message: format!("missing reverse dependency edge {dependent:?}"),
            });
        }

        if dependents.is_empty() {
            self.dependents.remove(&owner);
        } else {
            self.dependents.insert(owner, dependents);
        }

        Ok(())
    }
}

/// One compact artifact binding selection in a revision.
#[derive(Debug)]
pub(crate) struct ArtifactState {
    /// The immutable repository-wide binding.
    pub(crate) binding: ArtifactBindingId,
    /// Dependency ordinals that may have changed.
    pub(crate) dirty_dependencies: SmallVec<[u32; 2]>,
}

impl ArtifactState {
    /// Return whether this artifact has no dirty dependencies.
    pub(crate) fn is_clean(&self) -> bool {
        self.dirty_dependencies.is_empty()
    }
}

/// One value that can invalidate direct artifact dependents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ArtifactDependencyOwner {
    /// Another semantic artifact.
    Artifact(ArtifactId),
    /// One source file.
    File(FileId),
    /// The repository package set.
    Packages,
    /// The repository module set.
    Modules,
}

impl ArtifactDependencyOwner {
    /// Select the reverse dependency owner for one exact observation.
    fn new(
        dependency: &ArtifactDependency,
        artifact_id: impl Fn(ArtifactKey) -> Option<ArtifactId>,
    ) -> Result<Self, RepositoryError> {
        match dependency {
            ArtifactDependency::Artifact(version) => {
                let Some(artifact) = artifact_id(version.key) else {
                    return Err(RepositoryError::MissingArtifactId { key: version.key });
                };

                Ok(Self::Artifact(artifact))
            }
            ArtifactDependency::Projection(dependency) => {
                let key = dependency.projection().artifact;
                let Some(artifact) = artifact_id(key) else {
                    return Err(RepositoryError::MissingArtifactId { key });
                };

                Ok(Self::Artifact(artifact))
            }
            ArtifactDependency::Source(SourceDependency::FileContent { file, .. }) => {
                Ok(Self::File(*file))
            }
            ArtifactDependency::Source(SourceDependency::Packages { .. }) => Ok(Self::Packages),
            ArtifactDependency::Source(SourceDependency::Modules { .. }) => Ok(Self::Modules),
        }
    }
}

/// One reverse dependency edge into an artifact binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ArtifactDependent {
    /// The dependent artifact id.
    artifact: ArtifactId,
    /// The dependency ordinal inside the dependent binding.
    dependency: u32,
}
