use std::sync::Arc;

use destack_artifact::{ArtifactDependency, ArtifactKey, ArtifactPayload};
use destack_source::{File, FileId};
use destack_workspace::{ProviderContext, Revision};

use crate::{SessionError, SessionProviderContext, SessionState};

impl SessionState {
    /// Provide one source-derived artifact for a fixed revision.
    pub(crate) fn provide_source(
        &self,
        context: &SessionProviderContext,
    ) -> Result<ArtifactPayload, SessionError> {
        match context.artifact_key() {
            ArtifactKey::Ast { module } => self.provide_ast(module, context),
            ArtifactKey::Data { module } => self.provide_data(module, context),
            artifact_key => Err(SessionError::Internal {
                detail: format!("non source artifact reached source provider: {artifact_key:?}"),
            }),
        }
    }

    /// Load one source file and record its exact content dependency.
    pub(super) fn file(
        &self,
        revision: Revision,
        file_id: FileId,
        context: &SessionProviderContext,
    ) -> Result<Arc<File>, SessionError> {
        let content_id = self
            .repository()
            .file_content_id(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;
        let file = self
            .repository()
            .file(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        context.track(ArtifactDependency::file_content(file_id, content_id));

        Ok(file)
    }
}
