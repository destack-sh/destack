use std::collections::BTreeMap;
use std::sync::Arc;

use crate::artifact::ArtifactReader;
use crate::provider::ArtifactAttemptRecorder;
use crate::repository::{Repository, RepositoryError, Revision};
use crate::{ArtifactBase, ArtifactBindingState, ArtifactResolution};
use destack_artifact::{
    ArtifactBinding, ArtifactBindingId, ArtifactBindingPin, ArtifactDependency, ArtifactFailure,
    ArtifactFlush, ArtifactKey, ArtifactOutcome, ArtifactPayload, ArtifactRecord, ArtifactSidecar,
    ArtifactVersion,
};
use destack_source::DiagnosticCollection;
use rustc_hash::FxHashSet;

impl Repository {
    /// Return a read-only artifact reader for one pinned revision.
    pub fn artifact_reader(&self, revision: Revision) -> ArtifactReader<'_> {
        ArtifactReader::new(self, revision)
    }

    /// Return the fresh version for one revision-scoped artifact key.
    pub fn artifact_version(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let resolution = self.resolve_artifact(revision, artifact_key)?;
        let version = match resolution {
            ArtifactResolution::Terminal { version, .. } => Some(version),
            ArtifactResolution::Pending { .. } | ArtifactResolution::Stale => None,
        };

        Ok(version)
    }

    /// Return the terminal outcome for one exact current revision binding.
    pub fn current_artifact_outcome(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactOutcome>, RepositoryError> {
        let Some(state) = self.artifact_binding_state(revision, *artifact_key)? else {
            return Ok(None);
        };
        if !state.is_clean() {
            return Ok(None);
        }
        let binding = self.require_artifact_binding(state.binding)?;

        let outcome = self.artifact_table().outcome(&binding.version).ok_or(
            RepositoryError::MissingArtifact {
                version: binding.version,
            },
        )?;

        Ok(Some(outcome))
    }

    /// Return fresh versions for revision-scoped artifact keys.
    pub fn artifact_versions(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<Vec<Option<ArtifactVersion>>, RepositoryError> {
        let resolutions = self.resolve_artifacts(revision, artifact_keys)?;
        let mut versions = Vec::with_capacity(artifact_keys.len());

        // preserve request order
        for resolution in resolutions {
            let version = match resolution {
                ArtifactResolution::Terminal { version, .. } => Some(version),
                ArtifactResolution::Pending { .. } | ArtifactResolution::Stale => None,
            };

            versions.push(version);
        }

        Ok(versions)
    }

    /// Return the selected predecessor for one revision artifact.
    pub fn artifact_base(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Option<Arc<ArtifactBase>>, RepositoryError> {
        // fall back to the newest recorded binding of the key: any predecessor
        //  is a valid derivation base, providers verify against current inputs
        let state = match self.artifact_binding_state(revision, artifact_key)? {
            Some(state) => state,
            None => {
                let Some(artifact) = self.artifact_table().artifact_id(artifact_key) else {
                    return Ok(None);
                };
                let Some(binding) = self.artifact_table().latest_artifact_binding(artifact) else {
                    return Ok(None);
                };
                let recorded = self.require_artifact_binding(binding)?;

                // mark every observation dirty under a foreign revision
                ArtifactBindingState {
                    binding,
                    dirty_dependencies: (0..recorded.dependencies.len() as u32).collect(),
                }
            }
        };
        let binding = self.require_artifact_binding(state.binding)?;
        match self.artifact_table().outcome(&binding.version) {
            Some(ArtifactOutcome::Ok) => {}
            Some(ArtifactOutcome::Failed(_)) => return Ok(None),
            None => {
                return Err(RepositoryError::MissingArtifact {
                    version: binding.version,
                });
            }
        }

        // retain the selected predecessor binding
        let binding_pin = self.artifact_table().pin_binding(state.binding).ok_or(
            RepositoryError::MissingArtifactBindingId {
                binding: state.binding,
            },
        )?;
        let base = ArtifactBase::new(
            binding.version,
            binding.dependencies,
            state.dirty_dependencies,
            binding_pin,
        );

        Ok(Some(Arc::new(base)))
    }

    /// Derive the identity of one complete artifact dependency set.
    pub fn artifact_identity(
        &self,
        key: ArtifactKey,
        dependencies: &[ArtifactDependency],
    ) -> ArtifactVersion {
        ArtifactVersion::new(key, self.build_fingerprint(), dependencies.iter().cloned())
    }

    /// Bind one recorded artifact version to one revision when present.
    ///
    /// A hit proves the previous execution observed nothing beyond this set,
    /// so a deterministic provider re-run would reproduce it exactly.
    pub fn bind_artifact_version(
        &self,
        revision: Revision,
        key: ArtifactKey,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
    ) -> Result<bool, RepositoryError> {
        let dependencies = dependencies.into();
        let version = self.artifact_identity(key, &dependencies);
        if self.artifact_table().outcome(&version).is_none() {
            return Ok(false);
        }
        let binding = self
            .artifact_table()
            .publish_binding(version, dependencies)
            .map_err(|error| RepositoryError::InvalidArtifact {
                message: error.to_string(),
            })?;

        self.bind_artifact(revision, binding.binding())?;

        Ok(true)
    }

    /// Load one artifact from the persistent store when present.
    pub fn load_artifact(
        &self,
        version: ArtifactVersion,
    ) -> Result<Option<ArtifactPayload>, RepositoryError> {
        let Some((_record, payload)) = self.load_artifact_record(version)? else {
            return Ok(None);
        };

        Ok(Some(payload))
    }

    /// Load one persisted result and select its exact revision binding.
    pub fn load_artifact_binding(
        &self,
        revision: Revision,
        key: ArtifactKey,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        recorder: Option<&ArtifactAttemptRecorder>,
    ) -> Result<bool, RepositoryError> {
        let dependencies = dependencies.into();
        let version = self.artifact_identity(key, &dependencies);
        let Some((record, payload)) = self.load_artifact_record(version)? else {
            return Ok(false);
        };
        let binding = self.publish_artifact_result(
            version,
            payload,
            dependencies,
            record.diagnostics,
            record.sidecars,
            recorder,
        )?;
        self.bind_artifact(revision, binding.binding())?;

        Ok(true)
    }

    /// Complete one ready artifact and queue it for persistence.
    pub fn complete_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
        payload: ArtifactPayload,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        recorder: Option<&ArtifactAttemptRecorder>,
    ) -> Result<(), RepositoryError> {
        // derive the reusable identity of this dependency set
        let dependencies = dependencies.into();
        let identity = || self.artifact_identity(key, &dependencies);
        let version = match recorder {
            Some(recorder) => recorder.breakdown("commit.identity", identity),
            None => identity(),
        };

        // publish the immutable result and binding
        let binding_pin = self.publish_artifact_result(
            version,
            payload,
            Arc::clone(&dependencies),
            diagnostics,
            sidecars,
            recorder,
        )?;

        // select the published binding in this revision
        let bind = || self.bind_artifact(revision, binding_pin.binding());
        match recorder {
            Some(recorder) => recorder.breakdown("commit.bind", bind),
            None => bind(),
        }?;

        // queue the version for persistent storage
        let queue = || {
            self.pending_artifacts
                .entry(version)
                .or_insert(dependencies);
        };
        match recorder {
            Some(recorder) => recorder.breakdown("commit.queue", queue),
            None => queue(),
        };

        Ok(())
    }

    /// Flush pending artifacts into the persistent artifact store.
    pub fn flush_artifacts(&self) -> Result<ArtifactFlush, RepositoryError> {
        let pending = self
            .pending_artifacts
            .iter()
            .map(|entry| (*entry.key(), Arc::clone(entry.value())))
            .collect::<Vec<_>>();

        // encode and transfer every completed artifact version to the persistent store
        for (version, dependencies) in &pending {
            let record = self
                .artifact_table()
                .record(*version, dependencies, self.string_pool())
                .map_err(|error| RepositoryError::ArtifactStore {
                    message: error.to_string(),
                })?;
            self.artifact_store().store(record).map_err(|error| {
                RepositoryError::ArtifactStore {
                    message: error.to_string(),
                }
            })?;
            self.pending_artifacts.remove(version);
        }
        let flush = self
            .artifact_store()
            .flush(self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        Ok(flush)
    }

    /// Fail one artifact and bind its exact version to one revision.
    pub fn fail_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), RepositoryError> {
        let dependencies = dependencies.into();
        let version = self.artifact_identity(key, &dependencies);

        // store failure before exposing the revision binding
        let binding_pin = self
            .artifact_table()
            .fail(version, dependencies, diagnostics, sidecars, failure)
            .map_err(|error| RepositoryError::InvalidArtifact {
                message: error.to_string(),
            })?;
        self.bind_artifact(revision, binding_pin.binding())?;

        Ok(())
    }

    /// Return diagnostics for one revision, optionally filtered to one artifact key.
    pub fn diagnostics(
        &self,
        revision: Revision,
        artifact_key: Option<ArtifactKey>,
    ) -> Result<DiagnosticCollection, RepositoryError> {
        let mut diagnostics = DiagnosticCollection::new();

        // exact artifact
        if let Some(artifact_key) = artifact_key {
            let Some(version) = self.artifact_version(revision, &artifact_key)? else {
                return Ok(diagnostics);
            };
            let artifact_diagnostics = self.artifact_diagnostics(version)?;
            diagnostics.merge_from(artifact_diagnostics.as_ref());

            return Ok(diagnostics);
        }

        let revision_state = self.revision(revision)?;
        let bindings = revision_state.artifacts.bindings();
        let mut versions = Vec::with_capacity(bindings.len());
        for binding in bindings {
            versions.push(self.require_artifact_binding(binding)?.version);
        }

        // resolve every selected artifact key
        let keys = versions
            .iter()
            .map(|version| version.key)
            .collect::<Vec<_>>();
        let current_versions = self.artifact_versions(revision, &keys)?;

        // all fresh artifacts in this revision
        for (version, current_version) in versions.into_iter().zip(current_versions) {
            let Some(current_version) = current_version else {
                continue;
            };
            if current_version != version {
                continue;
            }

            let artifact_diagnostics = self.artifact_diagnostics(version)?;
            diagnostics.merge_from(artifact_diagnostics.as_ref());
        }

        Ok(diagnostics)
    }

    /// Return diagnostics for requested roots and everything they were built from.
    ///
    /// Walks each root's dependency closure once, so diagnostics attached to
    /// shared dependencies, like one checked component under its per-module
    /// facades, report exactly once.
    pub fn diagnostics_for_keys(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<DiagnosticCollection, RepositoryError> {
        let mut diagnostics = DiagnosticCollection::new();
        let mut merged = FxHashSet::default();
        let mut visited = FxHashSet::default();
        let mut pending = artifact_keys.to_vec();

        // walk exact revision bindings rather than conflating equal result versions
        while let Some(artifact_key) = pending.pop() {
            if !visited.insert(artifact_key) {
                continue;
            }
            let Some(version) = self.artifact_version(revision, &artifact_key)? else {
                continue;
            };
            if merged.insert(version) {
                let artifact_diagnostics = self.artifact_diagnostics(version)?;
                diagnostics.merge_from(artifact_diagnostics.as_ref());
            }

            let binding = self
                .artifact_binding(revision, &artifact_key)?
                .ok_or(RepositoryError::MissingArtifact { version })?;
            for dependency in binding.dependencies.iter() {
                match dependency {
                    ArtifactDependency::Artifact(version) => pending.push(version.key),
                    ArtifactDependency::Projection(projection) => {
                        pending.push(projection.projection().artifact);
                    }
                    ArtifactDependency::Source(_) => {}
                }
            }
        }

        Ok(diagnostics)
    }

    /// Return sidecars for one revision-scoped artifact key.
    pub fn artifact_sidecars(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Arc<[ArtifactSidecar]>, RepositoryError> {
        let Some(version) = self.artifact_version(revision, &artifact_key)? else {
            return Ok(Arc::from([]));
        };

        self.artifact_table()
            .sidecars(&version)
            .ok_or(RepositoryError::MissingArtifact { version })
    }

    /// Return one sidecar by exact name and label set.
    pub fn artifact_sidecar(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
        name: &str,
        labels: &BTreeMap<String, String>,
    ) -> Result<Option<ArtifactSidecar>, RepositoryError> {
        let sidecars = self.artifact_sidecars(revision, artifact_key)?;

        Ok(sidecars
            .iter()
            .find(|sidecar| sidecar.matches(name, labels))
            .cloned())
    }

    /// Return one exact revision artifact binding.
    pub(crate) fn artifact_binding(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactBinding>, RepositoryError> {
        let Some(state) = self.artifact_binding_state(revision, *artifact_key)? else {
            return Ok(None);
        };
        let binding = self.require_artifact_binding(state.binding)?;

        Ok(Some(binding))
    }

    /// Return one revision artifact state.
    fn artifact_binding_state(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Option<ArtifactBindingState>, RepositoryError> {
        let revision = self.revision(revision)?;
        let Some(artifact) = self.artifact_table().artifact_id(artifact_key) else {
            return Ok(None);
        };
        let Some(state) = revision.artifacts.state(artifact) else {
            return Ok(None);
        };

        Ok(Some(state))
    }

    /// Bind one immutable repository artifact result to one revision.
    fn bind_artifact(
        &self,
        revision: Revision,
        binding_id: ArtifactBindingId,
    ) -> Result<(), RepositoryError> {
        let revision = self.revision(revision)?;
        let binding = self.require_artifact_binding(binding_id)?;
        let artifact = self
            .artifact_table()
            .artifact_id(binding.version.key)
            .ok_or(RepositoryError::MissingArtifactId {
                key: binding.version.key,
            })?;
        revision.artifacts.bind(artifact, binding_id);

        Ok(())
    }

    /// Return one immutable repository artifact binding.
    fn require_artifact_binding(
        &self,
        binding: ArtifactBindingId,
    ) -> Result<ArtifactBinding, RepositoryError> {
        self.artifact_table()
            .binding(binding)
            .ok_or(RepositoryError::MissingArtifactBindingId { binding })
    }

    /// Publish one immutable artifact result and binding.
    fn publish_artifact_result(
        &self,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: Arc<[ArtifactDependency]>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        recorder: Option<&ArtifactAttemptRecorder>,
    ) -> Result<ArtifactBindingPin, RepositoryError> {
        let load_contents = || self.load_artifact_contents(&payload);
        match recorder {
            Some(recorder) => recorder.breakdown("commit.retain", load_contents),
            None => load_contents(),
        }?;

        let publish = || {
            self.artifact_table()
                .publish(version, payload, dependencies, diagnostics, sidecars)
                .map_err(|error| RepositoryError::InvalidArtifact {
                    message: error.to_string(),
                })
        };
        let binding = match recorder {
            Some(recorder) => recorder.breakdown("commit.publish", publish),
            None => publish(),
        }?;

        Ok(binding)
    }

    /// Load and decode one persisted artifact record.
    fn load_artifact_record(
        &self,
        version: ArtifactVersion,
    ) -> Result<Option<(ArtifactRecord, ArtifactPayload)>, RepositoryError> {
        let Some(record) = self
            .artifact_store()
            .load(&version, self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?
        else {
            return Ok(None);
        };
        if record.version != version {
            return Err(RepositoryError::ArtifactStore {
                message: "artifact store returned a different version".to_owned(),
            });
        }
        let payload = record
            .decode(self.host().build_id(), self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;
        self.load_artifact_contents(&payload)?;

        Ok(Some((record, payload)))
    }

    /// Load all content ids referenced by one artifact payload.
    fn load_artifact_contents(&self, payload: &ArtifactPayload) -> Result<(), RepositoryError> {
        for content in payload.content_ids() {
            let _ = self.content(content)?;
        }

        Ok(())
    }

    /// Return diagnostics for one exact artifact version.
    fn artifact_diagnostics(
        &self,
        version: ArtifactVersion,
    ) -> Result<Arc<DiagnosticCollection>, RepositoryError> {
        self.artifact_table()
            .diagnostics(&version)
            .ok_or(RepositoryError::MissingArtifact { version })
    }
}
