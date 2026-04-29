use std::sync::{Arc, OnceLock};

use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactOutcome, ArtifactPayload,
    ArtifactVersion, ProvideError, ProviderContext, ProviderResult, RequireError,
};
use destack_source::DiagnosticCollection;
use destack_workspace::{Repository, Revision};
use parking_lot::Mutex;

use crate::SessionError;

/// One session-owned artifact provider attempt.
#[derive(Debug)]
pub(crate) struct SessionContext {
    /// The repository that owns the pinned revision.
    repository: Arc<Repository>,
    /// The pinned revision for this attempt.
    revision: Revision,
    /// The artifact key being built.
    artifact_key: ArtifactKey,
    /// The exact dependencies read by this attempt.
    dependencies: Mutex<Vec<ArtifactDependency>>,
    /// The diagnostics produced by this attempt.
    diagnostics: Mutex<DiagnosticCollection>,
    /// The typed payload produced by this attempt.
    payload: OnceLock<ArtifactPayload>,
}

impl SessionContext {
    /// Create one provider attempt context.
    pub(crate) fn new(
        repository: Arc<Repository>,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Self {
        Self {
            repository,
            revision,
            artifact_key,
            dependencies: Mutex::new(Vec::new()),
            diagnostics: Mutex::new(DiagnosticCollection::new()),
            payload: OnceLock::new(),
        }
    }

    /// Return the pinned repository revision for this attempt.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the artifact key being built.
    pub(crate) fn key(&self) -> ArtifactKey {
        self.artifact_key
    }

    /// Return the exact dependencies read by this attempt.
    pub(crate) fn dependencies(&self) -> Vec<ArtifactDependency> {
        self.dependencies.lock().clone()
    }

    /// Return diagnostics produced by this attempt.
    pub(crate) fn diagnostic_collection(&self) -> DiagnosticCollection {
        self.diagnostics.lock().clone()
    }

    /// Complete this attempt with its ready payload.
    pub(crate) fn complete_ready(&self) -> Result<ArtifactVersion, SessionError> {
        let dependencies = self.dependencies();
        let diagnostics = self.diagnostic_collection();

        // ready artifacts must have exactly one payload
        let Some(payload) = self.payload.get().cloned() else {
            return Err(SessionError::Internal {
                detail: format!(
                    "provider completed without recording an artifact payload: {:?}",
                    self.artifact_key
                ),
            });
        };

        let version = ArtifactVersion::new(self.artifact_key, dependencies.iter().cloned());

        // store the artifact image before binding it to the revision
        self.repository
            .artifact_store()
            .publish(version, payload, dependencies, diagnostics);
        self.repository
            .record_artifact_version(self.revision, version)?;

        Ok(version)
    }

    /// Complete this attempt with diagnostics and no payload.
    pub(crate) fn complete_diagnosed(&self) -> Result<ArtifactVersion, SessionError> {
        let dependencies = self.dependencies();
        let diagnostics = self.diagnostic_collection();

        let version = ArtifactVersion::new(self.artifact_key, dependencies.iter().cloned());

        // store the artifact image before binding it to the revision
        self.repository
            .artifact_store()
            .publish_diagnosed(version, dependencies, diagnostics);
        self.repository
            .record_artifact_version(self.revision, version)?;

        Ok(version)
    }

    /// Complete this attempt with a provider failure.
    pub(crate) fn complete_failed(
        &self,
        failure: ArtifactFailure,
    ) -> Result<ArtifactVersion, SessionError> {
        let dependencies = self.dependencies();
        let diagnostics = self.diagnostic_collection();

        let version = ArtifactVersion::new(self.artifact_key, dependencies.iter().cloned());

        // store the artifact image before binding it to the revision
        self.repository.artifact_store().publish_failed(
            version,
            dependencies,
            diagnostics,
            failure,
        );
        self.repository
            .record_artifact_version(self.revision, version)?;

        Ok(version)
    }

    /// Add one dependency if it has not already been added.
    fn add_dependency(&self, dependency: ArtifactDependency) {
        let mut dependencies = self.dependencies.lock();
        if !dependencies.contains(&dependency) {
            dependencies.push(dependency);
        }
    }
}

impl ProviderContext for SessionContext {
    type Revision = Revision;

    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision {
        self.revision()
    }

    /// Return the artifact key being built.
    fn key(&self) -> ArtifactKey {
        self.key()
    }

    /// Require one artifact and return its exact version when ready.
    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, RequireError> {
        // can't require self
        if key == self.artifact_key {
            return Err(RequireError::Failed { key });
        }

        let Ok(Some(version)) = self.repository.artifact_version(self.revision, &key) else {
            return Err(RequireError::blocked(key));
        };

        match self.repository.artifact_store().outcome(&version) {
            Some(ArtifactOutcome::Ready) => {}
            Some(ArtifactOutcome::Diagnosed | ArtifactOutcome::Failed(_)) => {
                self.add_dependency(ArtifactDependency::artifact(version));

                return Err(RequireError::Failed { key });
            }
            None => return Err(RequireError::blocked(key)),
        }

        self.add_dependency(ArtifactDependency::artifact(version));

        Ok(version)
    }

    /// Add one exact dependency read by this attempt.
    fn dependency(&self, dependency: ArtifactDependency) {
        self.add_dependency(dependency);
    }

    /// Add diagnostics produced by this attempt.
    fn diagnostics(&self, diagnostics: DiagnosticCollection) {
        if diagnostics.is_empty() {
            return;
        }

        self.diagnostics.lock().merge_from(&diagnostics);
    }

    /// Set the typed payload produced by this attempt.
    fn payload(&self, payload: ArtifactPayload) -> ProviderResult<()> {
        if self.payload.set(payload).is_err() {
            return Err(ProvideError::internal(format!(
                "provider produced multiple payloads for {:?}",
                self.artifact_key
            )));
        }

        Ok(())
    }
}
