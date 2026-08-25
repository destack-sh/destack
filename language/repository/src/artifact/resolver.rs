use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactEntry, ArtifactKey, ArtifactOutcome, ArtifactVersion,
    SourceDependency, SourceDependencyKey,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{Repository, RepositoryError, Revision, RevisionState};

/// Decision of one artifact in a repository revision.
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

/// Lazy artifact selection for one repository revision.
struct ArtifactResolver<'a> {
    /// The owning repository.
    repository: &'a Repository,
    /// The requested revision.
    revision: Revision,
    /// The requested revision state.
    revision_state: Arc<RevisionState>,
    /// Resolutions completed during this operation.
    resolutions: FxHashMap<ArtifactKey, ArtifactResolution>,
    /// Artifact keys on the active recursive path.
    resolving: FxHashSet<ArtifactKey>,
}

impl Repository {
    /// Resolve one artifact in a repository revision.
    pub fn resolve_artifact(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<ArtifactResolution, RepositoryError> {
        let revision_state = self.revision(revision)?;
        if let Some(entry) = revision_state.artifacts.read().current(*artifact_key) {
            return Ok(ArtifactResolution::Terminal {
                version: entry.version,
                outcome: entry.outcome(),
            });
        }

        ArtifactResolver::new(self, revision, revision_state).resolve(*artifact_key)
    }

    /// Resolve artifacts in request order.
    pub(crate) fn resolve_artifacts(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<Vec<ArtifactResolution>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let mut resolver = ArtifactResolver::new(self, revision, revision_state);
        let mut resolutions = Vec::with_capacity(artifact_keys.len());

        for artifact_key in artifact_keys {
            resolutions.push(resolver.resolve(*artifact_key)?);
        }

        Ok(resolutions)
    }

    /// Return the keys not yet terminal at one revision.
    pub fn unresolved_artifact_keys(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<Vec<ArtifactKey>, RepositoryError> {
        let resolutions = self.resolve_artifacts(revision, artifact_keys)?;
        let pending = artifact_keys
            .iter()
            .copied()
            .zip(resolutions)
            .filter_map(|(key, resolution)| {
                (!matches!(resolution, ArtifactResolution::Terminal { .. })).then_some(key)
            })
            .collect();

        Ok(pending)
    }
}

impl<'a> ArtifactResolver<'a> {
    /// Build one resolver over a repository revision.
    fn new(
        repository: &'a Repository,
        revision: Revision,
        revision_state: Arc<RevisionState>,
    ) -> Self {
        Self {
            repository,
            revision,
            revision_state,
            resolutions: FxHashMap::default(),
            resolving: FxHashSet::default(),
        }
    }

    /// Resolve one artifact from its inherited candidate.
    fn resolve(&mut self, key: ArtifactKey) -> Result<ArtifactResolution, RepositoryError> {
        if let Some(resolution) = self.resolutions.get(&key) {
            return Ok(resolution.clone());
        }
        let Some((candidate, dirty)) = self.revision_state.artifacts.read().candidate(key) else {
            return Ok(ArtifactResolution::Stale);
        };
        if dirty.is_empty() {
            return Ok(Self::terminal(&candidate));
        }
        if !self.resolving.insert(key) {
            return Err(RepositoryError::CircularArtifactDependency { key });
        }

        // prove only observations reached by the edit
        let mut frontier = Vec::new();
        let mut is_stale = false;
        for dependency in dirty {
            let dependency = candidate.dependencies.get(dependency as usize).ok_or_else(|| {
                RepositoryError::InvalidArtifact {
                    message: format!(
                        "dirty artifact dependency is out of bounds: key={key:?}, dependency={dependency}"
                    ),
                }
            })?;
            match dependency {
                ArtifactDependency::Source(source) => {
                    is_stale = !self.source_matches(source)?;
                }
                ArtifactDependency::Artifact(version) => match self.resolve(version.key)? {
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
                },
                ArtifactDependency::Projection(projection) => {
                    let projection_key = projection.projection();
                    match self.resolve(projection_key.artifact)? {
                        ArtifactResolution::Terminal {
                            version: current, ..
                        } => {
                            let fingerprint = self
                                .repository
                                .artifact_table()
                                .projection_fingerprint(&current, &projection_key)
                                .map_err(|error| RepositoryError::InvalidArtifact {
                                    message: error.to_string(),
                                })?
                                .ok_or_else(|| RepositoryError::InvalidArtifact {
                                    message: format!(
                                        "artifact projection is absent from its owner: {projection_key:?}"
                                    ),
                                })?;
                            is_stale = fingerprint != projection.fingerprint();
                        }
                        ArtifactResolution::Pending {
                            frontier: dependency_frontier,
                        } => {
                            frontier.extend(dependency_frontier);
                        }
                        ArtifactResolution::Stale => {
                            if self.artifact_was_removed(projection_key.artifact)? {
                                is_stale = true;
                            } else {
                                frontier.push(projection_key.artifact);
                            }
                        }
                    }
                }
            }

            if is_stale {
                break;
            }
        }
        self.resolving.remove(&key);

        // select the candidate after every reached observation matches
        let resolution = if is_stale {
            ArtifactResolution::Stale
        } else if frontier.is_empty() {
            self.revision_state
                .artifacts
                .write()
                .select(candidate.clone())?;

            Self::terminal(&candidate)
        } else {
            frontier.sort_unstable();
            frontier.dedup();

            ArtifactResolution::Pending { frontier }
        };
        self.resolutions.insert(key, resolution.clone());

        Ok(resolution)
    }

    /// Return one terminal resolution for an artifact result.
    fn terminal(entry: &ArtifactEntry) -> ArtifactResolution {
        ArtifactResolution::Terminal {
            version: entry.version,
            outcome: entry.outcome(),
        }
    }

    /// Return whether one primitive source observation matches this revision.
    fn source_matches(&self, dependency: &SourceDependency) -> Result<bool, RepositoryError> {
        let current = match dependency.key() {
            SourceDependencyKey::File(file) => {
                let Some(blob) = self.repository.file_blob(self.revision, file)? else {
                    return Ok(false);
                };

                SourceDependency::file(file, blob.id)
            }
            SourceDependencyKey::Packages => {
                let fingerprint = self.repository.packages_fingerprint(self.revision)?;

                SourceDependency::Packages { fingerprint }
            }
            SourceDependencyKey::Modules => {
                let fingerprint = self.repository.modules_fingerprint(self.revision)?;

                SourceDependency::Modules { fingerprint }
            }
            SourceDependencyKey::ModulePath(file) => {
                let module = self.repository.module_id_for_file(self.revision, file)?;

                SourceDependency::module_path(file, module)
            }
        };

        Ok(current == *dependency)
    }

    /// Return whether one module artifact belongs to a removed module.
    fn artifact_was_removed(&self, key: ArtifactKey) -> Result<bool, RepositoryError> {
        let Some(module) = key.module_id() else {
            return Ok(false);
        };
        let is_tracked = self.repository.module(self.revision, module)?.is_some();

        Ok(!is_tracked)
    }
}
