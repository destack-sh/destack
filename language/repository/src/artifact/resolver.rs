use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactEntry, ArtifactKey, ArtifactOutcome, ArtifactVersion,
    SourceDependency,
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
    /// Return whether one exact source observation still holds.
    pub(crate) fn source_dependency_holds(
        &self,
        revision: Revision,
        source: SourceDependency,
    ) -> Result<bool, RepositoryError> {
        match source {
            SourceDependency::Package { package, .. } => {
                Ok(self.package_dependency(revision, package)? == source)
            }
            SourceDependency::Module { module, .. } => {
                Ok(self.module_dependency(revision, module)? == source)
            }
            SourceDependency::File { file, blob } => {
                let current = self.file_blob(revision, file)?.map(|blob| blob.id);

                Ok(current == Some(blob))
            }
            SourceDependency::Packages { fingerprint } => {
                Ok(self.packages_fingerprint(revision)? == fingerprint)
            }
            SourceDependency::Modules { fingerprint } => {
                Ok(self.modules_fingerprint(revision)? == fingerprint)
            }
            SourceDependency::PackageModules {
                package,
                fingerprint,
            } => Ok(self.package_modules_fingerprint(revision, package)? == fingerprint),
            SourceDependency::ModulePath { file, .. } => {
                let module = self.module_id_for_file(revision, file)?;

                Ok(SourceDependency::module_path(file, module) == source)
            }
        }
    }

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

    /// Resolve one artifact from inherited or shared candidates.
    fn resolve(&mut self, key: ArtifactKey) -> Result<ArtifactResolution, RepositoryError> {
        // reuse decisions made during this resolution
        if let Some(resolution) = self.resolutions.get(&key) {
            return Ok(resolution.clone());
        }
        let inherited = self.revision_state.artifacts.read().candidate(key);
        if let Some((candidate, dirty)) = &inherited
            && dirty.is_empty()
        {
            return Ok(Self::terminal(candidate));
        }
        if !self.resolving.insert(key) {
            return Err(RepositoryError::CircularArtifactDependency { key });
        }

        // validate inherited candidates only along dependencies changed by edits
        let mut resolution = ArtifactResolution::Stale;
        if let Some((candidate, dirty)) = &inherited {
            resolution = self.resolve_candidate(candidate, dirty.iter().copied())?;
        }

        // validate complete dependencies before selecting another revision's result
        if matches!(resolution, ArtifactResolution::Stale) {
            for candidate in self.repository.artifact_table().candidates(key) {
                if inherited
                    .as_ref()
                    .is_some_and(|(entry, _)| entry.version == candidate.version)
                {
                    continue;
                }
                let dependencies = 0..candidate.dependencies.len() as u32;
                resolution = self.resolve_candidate(&candidate, dependencies)?;
                if !matches!(resolution, ArtifactResolution::Stale) {
                    break;
                }
            }
        }
        self.resolving.remove(&key);
        self.resolutions.insert(key, resolution.clone());

        Ok(resolution)
    }

    /// Validate the selected dependency observations of one retained result.
    fn resolve_candidate(
        &mut self,
        candidate: &Arc<ArtifactEntry>,
        dependencies: impl Iterator<Item = u32>,
    ) -> Result<ArtifactResolution, RepositoryError> {
        // resolve every observed input before selecting the result
        let key = candidate.version.key;
        let mut frontier = Vec::new();
        let mut is_stale = false;
        for dependency in dependencies {
            let dependency = candidate.dependencies.get(dependency as usize).ok_or_else(|| {
                RepositoryError::InvalidArtifact {
                    message: format!(
                        "dirty artifact dependency is out of bounds: key={key:?}, dependency={dependency}"
                    ),
                }
            })?;
            match dependency {
                ArtifactDependency::Source(source) => {
                    is_stale = !self
                        .repository
                        .source_dependency_holds(self.revision, *source)?;
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

        // select the candidate after every reached observation matches
        let resolution = if is_stale {
            ArtifactResolution::Stale
        } else if frontier.is_empty() {
            self.revision_state
                .artifacts
                .write()
                .select(candidate.clone())?;

            Self::terminal(candidate)
        } else {
            frontier.sort_unstable();
            frontier.dedup();

            ArtifactResolution::Pending { frontier }
        };

        Ok(resolution)
    }

    /// Return one terminal resolution for an artifact result.
    fn terminal(entry: &ArtifactEntry) -> ArtifactResolution {
        ArtifactResolution::Terminal {
            version: entry.version,
            outcome: entry.outcome(),
        }
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
