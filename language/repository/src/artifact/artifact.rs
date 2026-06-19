use std::collections::BTreeMap;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactFlush, ArtifactKey, ArtifactPayload,
    ArtifactRecord, ArtifactSidecar, ArtifactStore, ArtifactTable, ArtifactVersion,
    SourceDependency,
};
use destack_core::Treap;
use destack_source::DiagnosticCollection;

use crate::artifact::ArtifactReader;
use crate::repository::{Repository, RepositoryError, Revision};

/// Repository-owned artifact state tables and persistent store.
#[derive(Debug)]
pub(crate) struct Artifacts {
    /// Shared artifact version treap.
    pub(crate) versions: Treap<ArtifactKey, ArtifactVersion>,
    /// Shared typed derived artifact table.
    pub(crate) table: Arc<ArtifactTable>,
    /// Persistent artifact record store.
    pub(crate) store: Arc<dyn ArtifactStore>,
}

impl Artifacts {
    /// Create empty artifact state tables over one persistent store.
    pub(crate) fn new(store: Arc<dyn ArtifactStore>) -> Self {
        Self {
            versions: Treap::new(),
            table: Arc::new(ArtifactTable::default()),
            store,
        }
    }

    /// Replace the persistent artifact store.
    pub(crate) fn set_store(&mut self, store: Arc<dyn ArtifactStore>) {
        self.store = store;
    }
}

impl Repository {
    /// Return a read-only artifact reader for one pinned revision.
    pub fn artifact_reader(&self, revision: Revision) -> ArtifactReader<'_> {
        ArtifactReader::new(self, revision)
    }

    /// Return the recorded artifact version for one revision-scoped artifact key.
    pub fn artifact_version(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let Some(version) = self
            .artifacts
            .versions
            .get(revision_state.artifacts(), artifact_key)
        else {
            return Ok(None);
        };

        if self.artifact_table().outcome(&version).is_some() {
            if self.artifact_dependencies_match(revision, &version)? {
                Ok(Some(version))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Bind one already-stored artifact version to one revision.
    pub fn bind_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
    ) -> Result<(), RepositoryError> {
        let revision_state = self.revision(revision)?;

        // the version must already carry a terminal outcome to be reusable
        if self.artifact_table().outcome(&version).is_none() {
            return Err(RepositoryError::MissingArtifact { version });
        }

        revision_state.bind_artifact(version, &self.artifacts.versions);

        Ok(())
    }

    /// Load one ready artifact from the persistent artifact store when present.
    pub fn load_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
    ) -> Result<bool, RepositoryError> {
        let Some(record) = self
            .artifact_store()
            .load(&version, self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?
        else {
            return Ok(false);
        };

        self.load_artifact_record(revision, record)?;

        Ok(true)
    }

    /// Publish one ready artifact and bind its exact version to one revision.
    pub fn complete_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        self.publish_artifact(
            revision,
            version,
            payload,
            dependencies,
            diagnostics,
            sidecars,
        )?;
        self.store_artifact(version)?;

        Ok(())
    }

    /// Publish one ready artifact and bind its exact version to one revision.
    pub fn publish_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        self.publish_ready_artifact(
            revision,
            version,
            payload,
            dependencies,
            diagnostics,
            sidecars,
        )
    }

    /// Publish one loaded artifact record and bind its exact version to one revision.
    fn load_artifact_record(
        &self,
        revision: Revision,
        record: ArtifactRecord,
    ) -> Result<(), RepositoryError> {
        let payload = record
            .decode_payload()
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        let _revision = self.revision(revision)?;
        self.load_artifact_contents(&payload)?;

        self.publish_ready_artifact(
            revision,
            record.version,
            payload,
            record.dependencies,
            record.diagnostics,
            record.sidecars,
        )
    }

    /// Publish one ready payload without writing the persistent store.
    fn publish_ready_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<(), RepositoryError> {
        let revision_state = self.revision(revision)?;
        self.load_artifact_contents(&payload)?;

        self.artifact_table()
            .publish(version, payload, dependencies, diagnostics, sidecars);
        revision_state.bind_artifact(version, &self.artifacts.versions);

        Ok(())
    }

    /// Store one ready artifact in the persistent artifact store.
    pub fn store_artifact(&self, version: ArtifactVersion) -> Result<(), RepositoryError> {
        self.artifact_store()
            .store(&version, self.artifact_table(), self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        Ok(())
    }

    /// Flush pending artifacts into the persistent artifact store.
    pub fn flush_artifacts(&self) -> Result<ArtifactFlush, RepositoryError> {
        let flush = self
            .artifact_store()
            .flush(self.string_pool())
            .map_err(|error| RepositoryError::ArtifactStore {
                message: error.to_string(),
            })?;

        Ok(flush)
    }

    /// Load all content ids referenced by one artifact payload.
    fn load_artifact_contents(&self, payload: &ArtifactPayload) -> Result<(), RepositoryError> {
        for content in payload.content_ids() {
            let _ = self.content(content)?;
        }

        Ok(())
    }

    /// Fail one artifact and bind its exact version to one revision.
    pub fn fail_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> Result<(), RepositoryError> {
        let revision_state = self.revision(revision)?;

        // store failure before exposing the revision binding
        self.artifact_table()
            .fail(version, dependencies, diagnostics, sidecars, failure);
        revision_state.bind_artifact(version, &self.artifacts.versions);

        Ok(())
    }

    /// Return diagnostics for one revision, optionally filtered to one artifact key.
    pub fn diagnostics(
        &self,
        revision: Revision,
        artifact_key: Option<ArtifactKey>,
    ) -> Result<DiagnosticCollection, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let mut diagnostics = DiagnosticCollection::new();

        // exact artifact
        if let Some(artifact_key) = artifact_key {
            let Some(version) = self.artifact_version(revision, &artifact_key)? else {
                return Ok(diagnostics);
            };
            let artifact_diagnostics = self
                .artifact_table()
                .diagnostics(&version)
                .map(|diagnostics| diagnostics.as_ref().clone())
                .ok_or(RepositoryError::MissingArtifact { version })?;

            diagnostics.merge_from(&artifact_diagnostics);

            return Ok(diagnostics);
        }

        let mut versions = Vec::new();
        self.artifacts
            .versions
            .visit(revision_state.artifacts(), &mut |_key, version| {
                versions.push(*version);
            });

        // all valid artifacts in this revision
        for version in versions {
            let Some(valid_version) = self.artifact_version(revision, &version.key)? else {
                continue;
            };
            if valid_version != version {
                continue;
            }

            let artifact_diagnostics = self
                .artifact_table()
                .diagnostics(&version)
                .map(|diagnostics| diagnostics.as_ref().clone())
                .ok_or(RepositoryError::MissingArtifact { version })?;
            diagnostics.merge_from(&artifact_diagnostics);
        }

        Ok(diagnostics)
    }

    /// Return sidecars for one revision-scoped artifact key.
    pub fn artifact_sidecars(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<Arc<[ArtifactSidecar]>, RepositoryError> {
        let _revision = self.revision(revision)?;
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

    /// Return whether one bound artifact version still matches one revision.
    fn artifact_dependencies_match(
        &self,
        revision: Revision,
        version: &ArtifactVersion,
    ) -> Result<bool, RepositoryError> {
        let dependencies = self
            .artifact_table()
            .dependencies(version)
            .ok_or(RepositoryError::MissingArtifact { version: *version })?;

        for dependency in dependencies.iter() {
            if !self.artifact_dependency_matches(revision, dependency)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return whether one exact artifact dependency still matches one revision.
    fn artifact_dependency_matches(
        &self,
        revision: Revision,
        dependency: &ArtifactDependency,
    ) -> Result<bool, RepositoryError> {
        match dependency {
            ArtifactDependency::Artifact(dependency) => {
                Ok(self.artifact_version(revision, &dependency.key)? == Some(*dependency))
            }
            ArtifactDependency::Source(dependency) => {
                self.source_dependency_matches(revision, dependency)
            }
        }
    }

    /// Return whether one exact source dependency still matches one revision.
    fn source_dependency_matches(
        &self,
        revision: Revision,
        dependency: &SourceDependency,
    ) -> Result<bool, RepositoryError> {
        match dependency {
            SourceDependency::FileContent { file, content } => {
                Ok(self.file_content_id(revision, *file)? == Some(*content))
            }
            SourceDependency::PathState { path, state } => {
                Ok(self.source_path_state(revision, *path)? == *state)
            }
            SourceDependency::DirectoryEntries { directory, entries } => {
                Ok(self.source_directory_entries(revision, *directory)? == *entries)
            }
        }
    }
}
