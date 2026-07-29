use std::collections::BTreeMap;
use std::sync::Arc;

use crate::artifact::ArtifactReader;
use crate::repository::{Repository, RepositoryError, Revision};
use crate::{ArtifactBase, ArtifactResolution, ArtifactState};
use destack_artifact::{
    ArtifactBinding, ArtifactBindingId, ArtifactBindingPin, ArtifactDependency, ArtifactFailure,
    ArtifactFlush, ArtifactInput, ArtifactKey, ArtifactOutcome, ArtifactPayload, ArtifactSidecar,
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
        let versions = self.artifact_versions(revision, std::slice::from_ref(artifact_key))?;

        Ok(versions.into_iter().next().flatten())
    }

    /// Return the exact bound version for one revision-scoped artifact key.
    pub fn bound_artifact_version(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let Some(state) = self.artifact_state(revision, *artifact_key)? else {
            return Ok(None);
        };
        let binding = self.require_artifact_binding(state.binding)?;
        let version = binding.version;
        if self.artifact_table().outcome(&version).is_none() {
            return Err(RepositoryError::MissingArtifact { version });
        }

        Ok(Some(version))
    }

    /// Return the terminal outcome for one exact current revision binding.
    pub fn current_artifact_outcome(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactOutcome>, RepositoryError> {
        let Some(state) = self.artifact_state(revision, *artifact_key)? else {
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
        self: &Arc<Self>,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Option<Arc<ArtifactBase>>, RepositoryError> {
        let Some(state) = self.artifact_state(revision, artifact_key)? else {
            return Ok(None);
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

    /// Bind one recorded artifact input to one revision when present.
    pub fn bind_artifact_input(
        &self,
        revision: Revision,
        input: ArtifactInput,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let Some(binding_id) = self.artifact_table().binding_id(&input) else {
            return Ok(None);
        };
        let binding_pin = self.artifact_table().pin_binding(binding_id).ok_or(
            RepositoryError::MissingArtifactBindingId {
                binding: binding_id,
            },
        )?;
        let binding = self.require_artifact_binding(binding_id)?;
        if self.artifact_table().outcome(&binding.version).is_none() {
            return Err(RepositoryError::MissingArtifact {
                version: binding.version,
            });
        }

        self.bind_artifact(revision, binding_pin.binding())?;

        Ok(Some(binding.version))
    }

    /// Load one ready artifact result from the persistent store when present.
    pub fn load_artifact(&self, version: ArtifactVersion) -> Result<bool, RepositoryError> {
        let Some(result) = self
            .artifact_store()
            .load_result(&version, self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?
        else {
            return Ok(false);
        };
        if result.version != version {
            return Err(RepositoryError::ArtifactStore {
                message: "artifact store returned a different result version".to_owned(),
            });
        }
        let payload = result
            .decode()
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        self.artifact_table()
            .publish_result(result.version, payload, result.diagnostics, result.sidecars)
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        Ok(true)
    }

    /// Load one artifact binding and result by exact input identity.
    pub fn load_artifact_input(
        &self,
        revision: Revision,
        input: ArtifactInput,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let Some(binding) = self
            .artifact_store()
            .load_binding(&input, self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?
        else {
            return Ok(None);
        };
        if binding.input != input {
            return Err(RepositoryError::ArtifactStore {
                message: "artifact store returned a different input binding".to_owned(),
            });
        }
        let Some(result) = self
            .artifact_store()
            .load_result(&binding.version, self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?
        else {
            return Err(RepositoryError::ArtifactStore {
                message: "artifact binding references a missing result".to_owned(),
            });
        };
        let payload = result
            .decode()
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        let (published, binding_pin) = self.publish_artifact_result(
            binding.input,
            payload,
            binding.dependencies.into(),
            result.diagnostics,
            result.sidecars,
        )?;
        if published != binding.version {
            return Err(RepositoryError::ArtifactStore {
                message: "persisted artifact result changed while publishing".to_owned(),
            });
        }
        self.bind_artifact(revision, binding_pin.binding())?;

        Ok(Some(binding.version))
    }

    /// Complete one ready artifact and queue it for persistence.
    pub fn complete_artifact(
        &self,
        revision: Revision,
        input: ArtifactInput,
        payload: ArtifactPayload,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<ArtifactVersion, RepositoryError> {
        // publish the live result before encoding its persistent record
        let version = self.publish_ready_artifact(
            revision,
            input,
            payload,
            dependencies.into(),
            diagnostics,
            sidecars,
        )?;
        self.pending_artifact_inputs.lock().insert(input);

        Ok(version)
    }

    /// Flush pending artifacts into the persistent artifact store.
    pub fn flush_artifacts(&self) -> Result<ArtifactFlush, RepositoryError> {
        let pending = self
            .pending_artifact_inputs
            .lock()
            .iter()
            .copied()
            .collect::<Vec<_>>();

        // encode and transfer every completed artifact input to the persistent store
        for input in &pending {
            let record = self
                .artifact_table()
                .record(input, self.string_pool())
                .map_err(|error| RepositoryError::ArtifactStore {
                    message: error.to_string(),
                })?
                .ok_or(RepositoryError::MissingArtifactInput { input: *input })?;
            self.artifact_store().store(record).map_err(|error| {
                RepositoryError::ArtifactStore {
                    message: error.to_string(),
                }
            })?;
            self.pending_artifact_inputs.lock().remove(input);
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
        input: ArtifactInput,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), RepositoryError> {
        let dependencies = dependencies.into();

        // store failure before exposing the revision binding
        let (_version, binding_pin) = self
            .artifact_table()
            .fail(input, dependencies, diagnostics, sidecars, failure)
            .map_err(|error| RepositoryError::ArtifactStore {
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
        let bindings = revision_state
            .artifacts
            .read()
            .bindings()
            .collect::<Vec<_>>();
        let mut versions = Vec::with_capacity(bindings.len());
        for binding in bindings {
            versions.push(self.require_artifact_binding(binding)?.version);
        }

        // resolve every selected key against one captured artifact graph
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
        let Some(state) = self.artifact_state(revision, *artifact_key)? else {
            return Ok(None);
        };
        let binding = self.require_artifact_binding(state.binding)?;

        Ok(Some(binding))
    }

    /// Return one revision artifact state.
    fn artifact_state(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Option<ArtifactState>, RepositoryError> {
        let revision = self.revision(revision)?;
        let Some(artifact) = self.artifact_table().artifact_id(artifact_key) else {
            return Ok(None);
        };
        let artifacts = revision.artifacts.read();
        let Some(state) = artifacts.state(artifact) else {
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
        let mut artifacts = revision.artifacts.write();
        let previous = artifacts
            .binding(artifact)
            .map(|binding| self.require_artifact_binding(binding))
            .transpose()?;
        artifacts.bind(
            artifact,
            binding_id,
            previous
                .as_ref()
                .map(|binding| binding.dependencies.as_ref()),
            &binding.dependencies,
            |key| self.artifact_table().artifact_id(key),
        )?;

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

    /// Publish one ready payload without writing the persistent store.
    fn publish_ready_artifact(
        &self,
        revision: Revision,
        input: ArtifactInput,
        payload: ArtifactPayload,
        dependencies: Arc<[ArtifactDependency]>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<ArtifactVersion, RepositoryError> {
        let (version, binding_pin) =
            self.publish_artifact_result(input, payload, dependencies, diagnostics, sidecars)?;
        self.bind_artifact(revision, binding_pin.binding())?;

        Ok(version)
    }

    /// Publish one immutable result and its artifact input entry.
    fn publish_artifact_result(
        &self,
        input: ArtifactInput,
        payload: ArtifactPayload,
        dependencies: Arc<[ArtifactDependency]>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(ArtifactVersion, ArtifactBindingPin), RepositoryError> {
        self.load_artifact_contents(&payload)?;

        let publication = self
            .artifact_table()
            .publish(input, payload, dependencies, diagnostics, sidecars)
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        Ok(publication)
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
