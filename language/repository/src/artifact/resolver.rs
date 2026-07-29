use destack_artifact::{
    ArtifactBinding, ArtifactBindingId, ArtifactDependency, ArtifactId, ArtifactKey,
    ArtifactOutcome, ArtifactVersion, ModuleSetFingerprint, PackageSetFingerprint,
    SourceDependency,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{ArtifactGraph, Repository, RepositoryError, Revision, RevisionState};

/// Resolution of one artifact in a repository revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactResolution {
    /// The artifact has a terminal result.
    Terminal {
        /// The resolved artifact version.
        version: ArtifactVersion,
        /// The terminal artifact outcome.
        outcome: ArtifactOutcome,
    },
    /// The artifact awaits dependencies.
    Pending {
        /// The lower artifact keys that must become terminal.
        frontier: Vec<ArtifactKey>,
    },
    /// The artifact is absent or stale.
    Stale,
}

/// One sparse dirty artifact resolution.
struct ArtifactResolver<'a> {
    /// The owning repository.
    repository: &'a Repository,
    /// The requested revision.
    revision: Revision,
    /// The requested revision state.
    revision_state: &'a RevisionState,
    /// The persistent graph root captured for this resolution.
    graph: ArtifactGraph,
    /// The captured revision graph generation.
    generation: u64,
    /// Dirty resolutions completed during this operation.
    resolutions: FxHashMap<ArtifactId, ArtifactResolution>,
    /// Artifact ids on the active recursive path.
    resolving: FxHashSet<ArtifactId>,
    /// Unchanged dirty artifact ids to publish.
    refreshed: Vec<ArtifactId>,
}

impl Repository {
    /// Resolve one artifact in a repository revision.
    pub fn resolve_artifact(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<ArtifactResolution, RepositoryError> {
        loop {
            let revision_state = self.revision(revision)?;
            let mut resolver = ArtifactResolver::new(self, revision, &revision_state);
            let resolution = resolver.resolve_key(*artifact_key)?;

            if resolver.commit()? {
                return Ok(resolution);
            }
        }
    }

    /// Resolve artifacts in request order.
    pub(crate) fn resolve_artifacts(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<Vec<ArtifactResolution>, RepositoryError> {
        loop {
            let revision_state = self.revision(revision)?;
            let mut resolver = ArtifactResolver::new(self, revision, &revision_state);
            let mut resolutions = Vec::with_capacity(artifact_keys.len());

            // resolve every requested key against one persistent graph root
            for artifact_key in artifact_keys {
                resolutions.push(resolver.resolve_key(*artifact_key)?);
            }

            if resolver.commit()? {
                return Ok(resolutions);
            }
        }
    }
}

impl<'a> ArtifactResolver<'a> {
    /// Build one resolver over a captured revision artifact graph.
    fn new(
        repository: &'a Repository,
        revision: Revision,
        revision_state: &'a RevisionState,
    ) -> Self {
        let graph = revision_state.artifacts.read();
        let generation = graph.generation();
        let graph = graph.clone();

        Self {
            repository,
            revision,
            revision_state,
            graph,
            generation,
            resolutions: FxHashMap::default(),
            resolving: FxHashSet::default(),
            refreshed: Vec::new(),
        }
    }

    /// Resolve one semantic artifact key.
    fn resolve_key(
        &mut self,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactResolution, RepositoryError> {
        let Some(artifact) = self.repository.artifact_table().artifact_id(artifact_key) else {
            return Ok(ArtifactResolution::Stale);
        };

        self.resolve(artifact)
    }

    /// Resolve one binding from only its dirty direct observations.
    fn resolve(&mut self, artifact: ArtifactId) -> Result<ArtifactResolution, RepositoryError> {
        if let Some(resolution) = self.resolutions.get(&artifact) {
            return Ok(resolution.clone());
        }
        let Some(state) = self.graph.state(artifact) else {
            return Ok(ArtifactResolution::Stale);
        };
        let binding = self.binding(state.binding)?;

        // return current bindings without allocating resolver state
        if state.is_clean() {
            return self.terminal(&binding);
        }
        if !self.resolving.insert(artifact) {
            return Err(RepositoryError::CircularArtifactBinding {
                key: binding.version.key,
            });
        }

        let mut frontier = Vec::new();
        let mut is_stale = false;

        // compare only observations reached by the source edit
        for dependency in &state.dirty_dependencies {
            let dependency = *dependency as usize;
            let observation = binding.dependencies.get(dependency).ok_or(
                RepositoryError::MissingArtifactDependency {
                    key: binding.version.key,
                    dependency,
                },
            )?;

            match observation {
                ArtifactDependency::Artifact(version) => {
                    let owner = self.artifact_id(version.key)?;

                    match self.resolve(owner)? {
                        ArtifactResolution::Terminal {
                            version: current, ..
                        } => {
                            is_stale = current != *version;
                        }
                        ArtifactResolution::Pending {
                            frontier: dependency_frontier,
                        } => {
                            frontier.extend(dependency_frontier);
                        }
                        ArtifactResolution::Stale => {
                            if self.artifact_was_removed(version.key)? {
                                is_stale = true;
                            } else {
                                frontier.push(version.key);
                            }
                        }
                    }
                }
                ArtifactDependency::Projection(projection) => {
                    let owner_key = projection.projection().artifact;
                    let owner = self.artifact_id(owner_key)?;

                    match self.resolve(owner)? {
                        ArtifactResolution::Terminal {
                            version: current, ..
                        } => {
                            let fingerprint = self
                                .repository
                                .artifact_table()
                                .projection_fingerprint(&current, &projection.projection());
                            is_stale = fingerprint != Some(projection.fingerprint());
                        }
                        ArtifactResolution::Pending {
                            frontier: dependency_frontier,
                        } => {
                            frontier.extend(dependency_frontier);
                        }
                        ArtifactResolution::Stale => {
                            if self.artifact_was_removed(owner_key)? {
                                is_stale = true;
                            } else {
                                frontier.push(owner_key);
                            }
                        }
                    }
                }
                ArtifactDependency::Source(source) => {
                    is_stale = !self.source_matches(source)?;
                }
            }

            if is_stale {
                break;
            }
        }

        self.resolving.remove(&artifact);

        // classify the dirty binding
        let resolution = if is_stale {
            ArtifactResolution::Stale
        } else if !frontier.is_empty() {
            frontier.sort_unstable();
            frontier.dedup();

            ArtifactResolution::Pending { frontier }
        } else {
            self.graph.refresh(artifact)?;
            self.refreshed.push(artifact);

            self.terminal(&binding)?
        };

        self.resolutions.insert(artifact, resolution.clone());

        Ok(resolution)
    }

    /// Return one immutable binding by compact id.
    fn binding(&self, binding: ArtifactBindingId) -> Result<ArtifactBinding, RepositoryError> {
        self.repository
            .artifact_table()
            .binding(binding)
            .ok_or(RepositoryError::MissingArtifactBindingId { binding })
    }

    /// Return one already interned artifact id.
    fn artifact_id(&self, key: ArtifactKey) -> Result<ArtifactId, RepositoryError> {
        self.repository
            .artifact_table()
            .artifact_id(key)
            .ok_or(RepositoryError::MissingArtifactId { key })
    }

    /// Return one terminal resolution for a current binding.
    fn terminal(&self, binding: &ArtifactBinding) -> Result<ArtifactResolution, RepositoryError> {
        let outcome = self
            .repository
            .artifact_table()
            .outcome(&binding.version)
            .ok_or(RepositoryError::MissingArtifact {
                version: binding.version,
            })?;

        Ok(ArtifactResolution::Terminal {
            version: binding.version,
            outcome,
        })
    }

    /// Return whether one primitive source observation matches this revision.
    fn source_matches(&self, dependency: &SourceDependency) -> Result<bool, RepositoryError> {
        match dependency {
            SourceDependency::FileContent { file, content } => {
                let current = self.repository.file_content_id(self.revision, *file)?;

                Ok(current == Some(*content))
            }
            SourceDependency::Packages { fingerprint } => {
                let packages = self.repository.package_ids(self.revision)?;
                let current = PackageSetFingerprint::new(&packages);

                Ok(&current == fingerprint)
            }
            SourceDependency::Modules { fingerprint } => {
                let modules = self.repository.module_ids(self.revision)?;
                let current = ModuleSetFingerprint::new(&modules);

                Ok(&current == fingerprint)
            }
        }
    }

    /// Return whether one module artifact belongs to a module removed from this revision.
    fn artifact_was_removed(&self, key: ArtifactKey) -> Result<bool, RepositoryError> {
        let Some(module) = key.module_id() else {
            return Ok(false);
        };
        let is_tracked = self.repository.module(self.revision, module)?.is_some();

        Ok(!is_tracked)
    }

    /// Publish unchanged dirty states when none were replaced concurrently.
    fn commit(self) -> Result<bool, RepositoryError> {
        if self.refreshed.is_empty() {
            let is_current = self.revision_state.artifacts.read().generation() == self.generation;

            return Ok(is_current);
        }

        let mut graph = self.revision_state.artifacts.write();
        if graph.generation() != self.generation {
            return Ok(false);
        }

        // refresh every unchanged selection
        for artifact in self.refreshed {
            graph.refresh(artifact)?;
        }

        Ok(true)
    }
}
