use std::sync::Arc;

use tspp_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload};
use tspp_repository::Revision;
use tspp_source::{File, FileId};

use crate::{ProviderAttempt, SessionError, SessionState};

impl SessionState {
    /// Collect the source closure for one loader-owned artifact.
    pub(crate) fn collect_loader(
        &self,
        attempt: &ProviderAttempt,
    ) -> Result<ArtifactDependencySet, SessionError> {
        match attempt.key() {
            ArtifactKey::DirParsed { module } => self.collect_dir_parsed(module, attempt),
            ArtifactKey::Data { module } => self.collect_data(module, attempt),
            artifact_key => Err(SessionError::Internal {
                detail: format!("non loader artifact reached loader collect: {artifact_key:?}"),
            }),
        }
    }

    /// Provide one loader-owned artifact for a fixed revision.
    pub(crate) fn provide_loader(
        &self,
        attempt: &ProviderAttempt,
    ) -> Result<ArtifactPayload, SessionError> {
        match attempt.key() {
            ArtifactKey::DirParsed { module } => self.provide_dir_parsed(module, attempt),
            ArtifactKey::Data { module } => self.provide_data(module, attempt),
            artifact_key => Err(SessionError::Internal {
                detail: format!("non loader artifact reached loader provider: {artifact_key:?}"),
            }),
        }
    }

    /// Observe one source file's exact content into a dependency closure.
    pub(super) fn observe_source(
        &self,
        revision: Revision,
        file_id: FileId,
        dependencies: &mut ArtifactDependencySet,
    ) -> Result<(), SessionError> {
        let blob = self
            .repository()
            .file_blob(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        dependencies.observe_file(file_id, blob.id);

        Ok(())
    }

    /// Load one tracked source file for a fixed revision.
    pub(super) fn source_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Arc<File>, SessionError> {
        self.repository()
            .file(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })
    }
}
