use std::sync::Arc;

use destack_artifact::{
    ArtifactBinding, ArtifactBindingId, ArtifactBindingPin, ArtifactDependency, ArtifactId,
    ArtifactKey, ArtifactOutcome, ArtifactVersion, ModuleSetFingerprint, PackageSetFingerprint,
    SourceDependency,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    ArtifactBindingState, ArtifactBindingTable, Repository, RepositoryError, Revision,
    RevisionState,
};

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
    /// Binding states observed during this resolution.
    observations: FxHashMap<ArtifactId, Option<ArtifactBindingState>>,
    /// Dirty resolutions completed during this operation.
    resolutions: FxHashMap<ArtifactId, ArtifactResolution>,
    /// Artifact ids on the active recursive path.
    resolving: FxHashSet<ArtifactId>,
    /// Exact bindings ready to commit for unchanged results.
    bindings: Vec<(ArtifactId, ArtifactBindingPin)>,
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
            if let Some(resolution) =
                self.clean_artifact_resolution(&revision_state.artifacts, *artifact_key)?
            {
                return Ok(resolution);
            }
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
            let mut clean = Vec::with_capacity(artifact_keys.len());

            // read clean selections directly
            for artifact_key in artifact_keys {
                let resolution =
                    self.clean_artifact_resolution(&revision_state.artifacts, *artifact_key)?;
                let Some(resolution) = resolution else {
                    clean.clear();
                    break;
                };
                clean.push(resolution);
            }
            if clean.len() == artifact_keys.len() {
                return Ok(clean);
            }

            // resolve all requested bindings against one observed state set
            let mut resolver = ArtifactResolver::new(self, revision, &revision_state);
            let mut resolutions = Vec::with_capacity(artifact_keys.len());
            for artifact_key in artifact_keys {
                resolutions.push(resolver.resolve_key(*artifact_key)?);
            }

            if resolver.commit()? {
                return Ok(resolutions);
            }
        }
    }

    /// Return a terminal or absent clean artifact selection.
    fn clean_artifact_resolution(
        &self,
        bindings: &ArtifactBindingTable,
        artifact_key: ArtifactKey,
    ) -> Result<Option<ArtifactResolution>, RepositoryError> {
        let Some(artifact) = self.artifact_table().artifact_id(artifact_key) else {
            return Ok(Some(ArtifactResolution::Stale));
        };
        let Some(state) = bindings.state(artifact) else {
            return Ok(Some(ArtifactResolution::Stale));
        };
        if !state.is_clean() {
            return Ok(None);
        }
        let binding = self.artifact_table().binding(state.binding).ok_or(
            RepositoryError::MissingArtifactBindingId {
                binding: state.binding,
            },
        )?;
        let outcome = self.artifact_table().outcome(&binding.version).ok_or(
            RepositoryError::MissingArtifact {
                version: binding.version,
            },
        )?;

        Ok(Some(ArtifactResolution::Terminal {
            version: binding.version,
            outcome,
        }))
    }
}

impl<'a> ArtifactResolver<'a> {
    /// Build one resolver over a repository revision.
    fn new(
        repository: &'a Repository,
        revision: Revision,
        revision_state: &'a RevisionState,
    ) -> Self {
        Self {
            repository,
            revision,
            revision_state,
            observations: FxHashMap::default(),
            resolutions: FxHashMap::default(),
            resolving: FxHashSet::default(),
            bindings: Vec::new(),
        }
    }

    /// Resolve one artifact key.
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
        let Some(state) = self.state(artifact) else {
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
        let mut dependencies = None;

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
                                .projection_fingerprint(&current, &projection.projection())
                                .ok_or_else(|| RepositoryError::InvalidArtifact {
                                    message: format!(
                                        "artifact projection is absent from its owner: {:?}",
                                        projection.projection()
                                    ),
                                })?;
                            is_stale = fingerprint != projection.fingerprint();
                            if !is_stale && current != projection.version() {
                                let dependencies = dependencies
                                    .get_or_insert_with(|| binding.dependencies.to_vec());
                                dependencies[dependency] = ArtifactDependency::projection(
                                    current,
                                    projection.projection().key,
                                    fingerprint,
                                );
                            }
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
            let resolution = self.terminal(&binding)?;
            let dependencies = match dependencies {
                Some(dependencies) => Arc::from(dependencies),
                None => Arc::clone(&binding.dependencies),
            };
            let binding = self
                .repository
                .artifact_table()
                .publish_binding(binding.version, dependencies)
                .map_err(|error| RepositoryError::InvalidArtifact {
                    message: error.to_string(),
                })?;
            self.bindings.push((artifact, binding));

            resolution
        };

        self.resolutions.insert(artifact, resolution.clone());

        Ok(resolution)
    }

    /// Read and retain one revision artifact binding state.
    fn state(&mut self, artifact: ArtifactId) -> Option<ArtifactBindingState> {
        if let Some(state) = self.observations.get(&artifact) {
            return state.clone();
        }

        let state = self.revision_state.artifacts.state(artifact);
        self.observations.insert(artifact, state.clone());

        state
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

    /// Publish unchanged dirty bindings when every observation still matches.
    fn commit(self) -> Result<bool, RepositoryError> {
        let bindings = self
            .bindings
            .iter()
            .map(|(artifact, binding)| (*artifact, binding.binding()))
            .collect::<Vec<_>>();

        self.revision_state
            .artifacts
            .commit(&self.observations, &bindings)
    }
}
