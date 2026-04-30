use std::sync::Arc;

use destack_artifact::{ArtifactDependency, ArtifactKey, ArtifactPayload, ProviderContext};
use destack_source::{File, FileId};
use destack_workspace::Revision;

use crate::SessionError;
use crate::session::{SessionContext, SessionState};

impl SessionState {
    /// Provide one source-derived artifact for a fixed revision.
    pub(crate) fn provide_source(
        &self,
        context: &SessionContext,
    ) -> Result<ArtifactPayload, SessionError> {
        match context.key() {
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
        context: &SessionContext,
    ) -> Result<Arc<File>, SessionError> {
        let content_id = self
            .repository()
            .file_content_id(revision, file_id)?
            .ok_or(SessionError::FileIdNotTracked { file_id })?;
        let file = self
            .repository()
            .file(revision, file_id)?
            .ok_or(SessionError::FileIdNotTracked { file_id })?;

        context.dependency(ArtifactDependency::file_content(file_id, content_id));

        Ok(file)
    }
}
