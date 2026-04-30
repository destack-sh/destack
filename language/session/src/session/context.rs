use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactOutcome, ArtifactPayload,
    ArtifactVersion, ProviderContext, RequireError,
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
    /// The (pinned) revision for this attempt.
    revision: Revision,
    /// The artifact key being built.
    key: ArtifactKey,
    /// The exact dependencies read by this attempt.
    dependencies: Mutex<Vec<ArtifactDependency>>,
    /// The diagnostics produced by this attempt.
    diagnostics: Mutex<DiagnosticCollection>,
}

impl SessionContext {
    /// Create one provider attempt context.
    pub(crate) fn new(repository: Arc<Repository>, revision: Revision, key: ArtifactKey) -> Self {
        Self {
            repository,
            revision,
            key,
            dependencies: Mutex::new(Vec::new()),
            diagnostics: Mutex::new(DiagnosticCollection::new()),
        }
    }

    /// Return the pinned repository revision for this attempt.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the artifact key being built.
    pub(crate) fn key(&self) -> ArtifactKey {
        self.key
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
    pub(crate) fn complete_ready(
        &self,
        payload: ArtifactPayload,
    ) -> Result<ArtifactVersion, SessionError> {
        let dependencies = self.dependencies();
        let diagnostics = self.diagnostic_collection();
        let version = ArtifactVersion::new(self.key, dependencies.iter().cloned());

        self.repository.publish_artifact(
            self.revision,
            version,
            payload,
            dependencies,
            diagnostics,
        )?;

        Ok(version)
    }

    /// Complete this attempt with a provider failure.
    pub(crate) fn complete_failed(
        &self,
        failure: ArtifactFailure,
    ) -> Result<ArtifactVersion, SessionError> {
        let dependencies = self.dependencies();
        let diagnostics = self.diagnostic_collection();

        let version = ArtifactVersion::new(self.key, dependencies.iter().cloned());

        self.repository.fail_artifact(
            self.revision,
            version,
            dependencies,
            diagnostics,
            failure,
        )?;

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
        if key == self.key {
            return Err(RequireError::Failed { key });
        }

        let Ok(Some(version)) = self.repository.artifact_version(self.revision, &key) else {
            return Err(RequireError::blocked(key));
        };

        match self.repository.artifact_store().outcome(&version) {
            Some(ArtifactOutcome::Ok) => {}
            Some(ArtifactOutcome::Failed(_)) => {
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
}
