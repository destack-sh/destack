use std::sync::Arc;

use destack_artifact::{ArtifactDependency, ArtifactKey, ArtifactPayload};
use destack_source::{File, FileId};
use destack_workspace::{ProviderContext, Revision};

use crate::{ProviderAttempt, SessionError, SessionState};

impl SessionState {
    /// Provide one loader-owned artifact for a fixed revision.
    pub(crate) fn provide_loader(
        &self,
        attempt: &ProviderAttempt,
    ) -> Result<ArtifactPayload, SessionError> {
        match attempt.key() {
            ArtifactKey::Ast { module } => self.provide_ast(module, attempt),
            ArtifactKey::Data { module } => self.provide_data(module, attempt),
            artifact_key => Err(SessionError::Internal {
                detail: format!("non loader artifact reached loader provider: {artifact_key:?}"),
            }),
        }
    }

    /// Load one source file and record its exact content dependency.
    pub(super) fn source_file(
        &self,
        revision: Revision,
        file_id: FileId,
        attempt: &ProviderAttempt,
    ) -> Result<Arc<File>, SessionError> {
        let repository = self.repository();
        let content_id = repository
            .file_content_id(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;
        let file = repository
            .file(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        attempt.track(ArtifactDependency::file_content(file_id, content_id));

        Ok(file)
    }
}
