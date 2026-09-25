use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::artifact::ArtifactReader;
use crate::provider::ArtifactAttemptRecorder;
use crate::repository::{Repository, RepositoryError, Revision};
use crate::{ArtifactBase, ArtifactResolution};
use rustc_hash::{FxHashSet, FxHasher};
use tspp_artifact::{
    ArtifactDependency, ArtifactEntry, ArtifactFailure, ArtifactKey, ArtifactOutcome,
    ArtifactPayload, ArtifactVersion, DeferredDiagnosticLabel, DiagnosticRecord, DirBound,
    DirParsed,
};
use tspp_core::Blob;
use tspp_source::{Diagnostic, DiagnosticCollection, DiagnosticLabel, DiagnosticTarget, FileId};

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

    /// Return the bound dependency keys of one revision-scoped artifact.
    pub fn artifact_dependency_keys(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Vec<ArtifactKey>, RepositoryError> {
        let Some(entry) = self.artifact_entry(revision, artifact_key)? else {
            return Ok(Vec::new());
        };

        let keys = entry
            .dependencies
            .iter()
            .filter_map(|dependency| match dependency {
                ArtifactDependency::Artifact(version) => Some(version.key),
                ArtifactDependency::Projection(projection) => {
                    Some(projection.projection().artifact)
                }
                ArtifactDependency::Source(_) => None,
            })
            .collect();

        Ok(keys)
    }

    /// Return the terminal outcome for one exact current revision artifact.
    pub fn current_artifact_outcome(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactOutcome>, RepositoryError> {
        let Some(entry) = self.artifact_entry(revision, artifact_key)? else {
            return Ok(None);
        };

        Ok(Some(entry.outcome()))
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

    /// Return one incremental base for a revision artifact.
    pub fn artifact_base(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Option<Arc<ArtifactBase>>, RepositoryError> {
        let revision = self.revision(revision)?;
        let selected = revision.artifacts.read().base(artifact_key);
        let selected = selected.filter(|entry| matches!(entry.outcome(), ArtifactOutcome::Ok));
        let entry = selected.or_else(|| self.artifact_table().base(artifact_key));
        let Some(entry) = entry else {
            return Ok(None);
        };
        let payload = self
            .artifact_table()
            .payload(&entry.version)
            .map_err(|error| RepositoryError::InvalidArtifact {
                message: error.to_string(),
            })?
            .ok_or(RepositoryError::MissingArtifact {
                version: entry.version,
            })?;

        let base = ArtifactBase::new(entry, payload);

        Ok(Some(Arc::new(base)))
    }

    /// Derive the identity of one complete artifact dependency set.
    pub fn artifact_identity(
        &self,
        key: ArtifactKey,
        dependencies: &[ArtifactDependency],
    ) -> ArtifactVersion {
        ArtifactVersion::new(key, self.host().build_id(), dependencies.iter().cloned())
    }

    /// Select one recorded artifact version in a revision when present.
    ///
    /// A hit proves the previous execution observed nothing beyond this set,
    /// so a deterministic provider re-run would reproduce it exactly.
    pub fn select_artifact_version(
        &self,
        revision: Revision,
        key: ArtifactKey,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
    ) -> Result<bool, RepositoryError> {
        let dependencies = dependencies.into();
        let version = self.artifact_identity(key, &dependencies);
        let Some(entry) = self.artifact_table().entry(&version) else {
            return Ok(false);
        };
        if entry.dependencies != dependencies {
            return Err(RepositoryError::InvalidArtifact {
                message: format!("artifact version has conflicting dependencies: {version:?}"),
            });
        }
        self.select_artifact(revision, entry)?;

        Ok(true)
    }

    /// Complete one ready artifact.
    pub fn complete_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
        payload: ArtifactPayload,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: Vec<DiagnosticRecord>,
        recorder: Option<&ArtifactAttemptRecorder>,
    ) -> Result<(), RepositoryError> {
        // derive the reusable identity of this dependency set
        let dependencies = dependencies.into();
        let identity = || self.artifact_identity(key, &dependencies);
        let version = match recorder {
            Some(recorder) => recorder.breakdown("commit.identity", identity),
            None => identity(),
        };

        // publish the immutable result
        let entry =
            self.publish_artifact_result(version, dependencies, payload, diagnostics, recorder)?;

        // select the published result in this revision
        let select = || self.select_artifact(revision, entry.clone());
        match recorder {
            Some(recorder) => recorder.breakdown("commit.select", select),
            None => select(),
        }?;

        Ok(())
    }

    /// Fail one artifact and select its exact version in one revision.
    pub fn fail_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: Vec<DiagnosticRecord>,
        failure: ArtifactFailure,
    ) -> Result<(), RepositoryError> {
        let dependencies = dependencies.into();
        let version = self.artifact_identity(key, &dependencies);

        // publish the failure before exposing the revision selection
        let entry = self
            .artifact_table()
            .fail(version, dependencies, diagnostics, failure);
        self.select_artifact(revision, entry)?;

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
            self.merge_resolved_diagnostics(revision, version, &mut diagnostics)?;

            return Ok(diagnostics);
        }

        let revision_state = self.revision(revision)?;
        let candidates = revision_state.artifacts.read().candidates();
        let versions = candidates
            .iter()
            .map(|entry| entry.version)
            .collect::<Vec<_>>();

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

            self.merge_resolved_diagnostics(revision, version, &mut diagnostics)?;
        }

        Ok(diagnostics)
    }

    /// Return diagnostics for requested roots and everything they were built from.
    pub fn diagnostics_for_keys(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<DiagnosticCollection, RepositoryError> {
        // serve the memoized closure for this exact key set
        let state = self.revision(revision)?;
        let mut requested_keys = artifact_keys.to_vec();
        requested_keys.sort_unstable();
        requested_keys.dedup();
        let mut hasher = FxHasher::default();
        requested_keys.hash(&mut hasher);
        let requested = hasher.finish();
        if let Some(diagnostics) = state.cache().diagnostics.lock().get(&requested) {
            return Ok(diagnostics.as_ref().clone());
        }

        let mut diagnostics = DiagnosticCollection::new();
        let mut merged = FxHashSet::default();
        let mut visited = FxHashSet::default();
        let mut is_terminal = true;
        let mut pending = requested_keys.clone();
        let mut requested_files = Vec::new();

        // walk exact revision artifacts and their recorded dependencies
        while let Some(artifact_key) = pending.pop() {
            if !visited.insert(artifact_key) {
                continue;
            }
            let Some(version) = self.artifact_version(revision, &artifact_key)? else {
                is_terminal = false;

                continue;
            };
            if merged.insert(version) {
                let first = diagnostics.diagnostics.len();
                self.merge_resolved_diagnostics(revision, version, &mut diagnostics)?;

                // rank the requested artifacts' files ahead of the dependencies' files
                if requested_keys.contains(&artifact_key) {
                    let files = diagnostics.diagnostics[first..]
                        .iter()
                        .map(|diagnostic| diagnostic.primary.target.file());
                    requested_files.extend(files);
                }
            }

            let entry = self
                .artifact_entry(revision, &artifact_key)?
                .ok_or(RepositoryError::MissingArtifact { version })?;
            for dependency in entry.dependencies.iter() {
                match dependency {
                    ArtifactDependency::Artifact(version) => pending.push(version.key),
                    ArtifactDependency::Projection(projection) => {
                        pending.push(projection.projection().artifact);
                    }
                    ArtifactDependency::Source(_) => {}
                }
            }
        }

        // order the closure by the requested artifacts' files, the dependencies' files after
        let rank = |file: FileId| {
            requested_files
                .iter()
                .position(|requested| *requested == file)
        };
        diagnostics.diagnostics.sort_by_key(|diagnostic| {
            let file = diagnostic.primary.target.file();
            let rank = rank(file);

            (rank.is_none(), rank, file)
        });

        // memoize only fully terminal closures, partial walks resolve further later
        if is_terminal {
            state
                .cache()
                .diagnostics
                .lock()
                .insert(requested, Arc::new(diagnostics.clone()));
        }

        Ok(diagnostics)
    }

    /// Return one artifact result proved current in a revision.
    fn artifact_entry(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<Arc<ArtifactEntry>>, RepositoryError> {
        let resolution = self.resolve_artifact(revision, artifact_key)?;
        let ArtifactResolution::Terminal { version, .. } = resolution else {
            return Ok(None);
        };
        let revision = self.revision(revision)?;
        let entry = revision
            .artifacts
            .read()
            .current(version.key)
            .ok_or(RepositoryError::MissingArtifact { version })?;

        Ok(Some(entry))
    }

    /// Select one immutable artifact result in a revision.
    fn select_artifact(
        &self,
        revision: Revision,
        entry: Arc<ArtifactEntry>,
    ) -> Result<(), RepositoryError> {
        let revision = self.revision(revision)?;

        revision.artifacts.write().select(entry)
    }

    /// Publish one immutable artifact result.
    fn publish_artifact_result(
        &self,
        version: ArtifactVersion,
        dependencies: Arc<[ArtifactDependency]>,
        payload: ArtifactPayload,
        diagnostics: Vec<DiagnosticRecord>,
        recorder: Option<&ArtifactAttemptRecorder>,
    ) -> Result<Arc<ArtifactEntry>, RepositoryError> {
        let mut blobs = payload.blobs();
        blobs.extend(
            diagnostics
                .iter()
                .flat_map(|record| record.diagnostic.blobs()),
        );
        blobs.sort_unstable();
        blobs.dedup();
        let load_contents = || self.require_blobs(&blobs);
        match recorder {
            Some(recorder) => recorder.breakdown("commit.retain", load_contents),
            None => load_contents(),
        }?;

        let publish = || {
            self.artifact_table()
                .publish(version, dependencies, payload, diagnostics)
                .map_err(|error| RepositoryError::InvalidArtifact {
                    message: error.to_string(),
                })
        };
        let entry = match recorder {
            Some(recorder) => recorder.breakdown("commit.publish", publish),
            None => publish(),
        }?;

        Ok(entry)
    }

    /// Require every exact Blob in one retained closure.
    fn require_blobs(&self, blobs: &[Blob]) -> Result<(), RepositoryError> {
        for blob in blobs {
            self.require_blob(*blob)?;
        }

        Ok(())
    }

    /// Return diagnostics for one exact artifact version.
    fn artifact_diagnostics(
        &self,
        version: ArtifactVersion,
    ) -> Result<Arc<[DiagnosticRecord]>, RepositoryError> {
        self.artifact_table()
            .diagnostics(&version)
            .ok_or(RepositoryError::MissingArtifact { version })
    }

    /// Merge one artifact's diagnostics, resolved onto current sources.
    fn merge_resolved_diagnostics(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        diagnostics: &mut DiagnosticCollection,
    ) -> Result<(), RepositoryError> {
        let records = self.artifact_diagnostics(version)?;
        for record in records.iter() {
            let resolved = self.resolve_record(revision, &version, record)?;
            diagnostics.diagnostics.push(resolved);
        }

        Ok(())
    }

    /// Resolve one stored diagnostic record onto current sources.
    fn resolve_record(
        &self,
        revision: Revision,
        version: &ArtifactVersion,
        record: &DiagnosticRecord,
    ) -> Result<Diagnostic, RepositoryError> {
        let mut diagnostic = record.diagnostic.clone();

        // append each deferred label at its current source span
        for deferred_label in &record.deferred_labels {
            let label = self.resolve_deferred_label(revision, version, deferred_label)?;
            diagnostic = diagnostic.label(label);
        }

        Ok(diagnostic)
    }

    /// Resolve one deferred diagnostic label onto its current source span.
    fn resolve_deferred_label(
        &self,
        revision: Revision,
        version: &ArtifactVersion,
        label: &DeferredDiagnosticLabel,
    ) -> Result<DiagnosticLabel, RepositoryError> {
        let profile = version
            .key
            .profile_id()
            .ok_or_else(|| RepositoryError::InvalidArtifact {
                message: format!("artifact has no profile for declaration label: {version:?}"),
            })?;
        let symbol = label.anchor;

        // resolve the declaration's local symbol entry
        let bound_key = ArtifactKey::dir_bound(symbol.module_id, profile);
        let bound_version = self
            .artifact_version(revision, &bound_key)?
            .ok_or_else(|| RepositoryError::InvalidArtifact {
                message: format!("current declaration has no bound artifact: {symbol:?}"),
            })?;
        let bound = self
            .artifact_table()
            .artifact::<DirBound>(&bound_version)
            .map_err(|error| RepositoryError::InvalidArtifact {
                message: error.to_string(),
            })?
            .ok_or_else(|| RepositoryError::InvalidArtifact {
                message: format!("bound artifact has no DIR payload: {bound_version:?}"),
            })?;
        let entry = bound
            .bindings
            .get_symbol_maybe(symbol.local_id)
            .ok_or_else(|| RepositoryError::InvalidArtifact {
                message: format!("bound artifact has no declaration symbol: {symbol:?}"),
            })?;
        let declaration = entry
            .declaration
            .ok_or_else(|| RepositoryError::InvalidArtifact {
                message: format!("bound symbol has no declaration: {symbol:?}"),
            })?;

        // resolve the declaration's span in the parsed tree
        let parsed_key = ArtifactKey::dir_parsed(symbol.module_id);
        let parsed_version = self
            .artifact_version(revision, &parsed_key)?
            .ok_or_else(|| RepositoryError::InvalidArtifact {
                message: format!("current declaration has no parsed artifact: {symbol:?}"),
            })?;
        let parsed = self
            .artifact_table()
            .artifact::<DirParsed>(&parsed_version)
            .map_err(|error| RepositoryError::InvalidArtifact {
                message: error.to_string(),
            })?
            .ok_or_else(|| RepositoryError::InvalidArtifact {
                message: format!("parsed artifact has no DIR payload: {parsed_version:?}"),
            })?;
        let span = parsed
            .tree
            .get_main_span_by_id(declaration.local_id.id)
            .or_else(|| parsed.tree.get_span_by_id(declaration.local_id.id))
            .ok_or_else(|| RepositoryError::InvalidArtifact {
                message: format!("parsed declaration has no span: {declaration:?}"),
            })?;
        let file = self
            .file(revision, span.file)?
            .ok_or(RepositoryError::InvalidFile {
                file: span.file,
                message: "declaration span references a missing file".to_string(),
            })?;

        Ok(DiagnosticLabel {
            blob: file.blob,
            target: DiagnosticTarget::Span(span),
            message: Some(label.message.clone()),
        })
    }
}
