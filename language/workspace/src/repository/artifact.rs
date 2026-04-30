use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactVersion,
};
use destack_source::DiagnosticCollection;

use crate::repository::{Repository, RepositoryError, Revision};

impl Repository {
    /// Return the recorded artifact version for one revision-scoped artifact key.
    pub fn artifact_version(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<ArtifactVersion>, RepositoryError> {
        let _revision = self.revision(revision)?;

        Ok(self
            .artifact_versions
            .get(&(revision, *artifact_key))
            .map(|version| *version.value()))
    }

    /// Publish one ready artifact and bind its exact version to one revision.
    pub fn complete_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;

        // store payload before exposing the revision binding
        self.artifact_store()
            .publish(version, payload, dependencies, diagnostics);
        self.artifact_versions
            .insert((revision, version.key), version);

        Ok(())
    }

    /// Fail one artifact and bind its exact version to one revision.
    pub fn fail_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        failure: ArtifactFailure,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;

        // store failure before exposing the revision binding
        self.artifact_store()
            .fail(version, dependencies, diagnostics, failure);
        self.artifact_versions
            .insert((revision, version.key), version);

        Ok(())
    }

    /// Return diagnostics for one exact revision scoped artifact key.
    pub fn artifact_diagnostics(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<DiagnosticCollection, RepositoryError> {
        let Some(version) = self.artifact_version(revision, artifact_key)? else {
            return Ok(DiagnosticCollection::new());
        };

        let diagnostics = self
            .artifact_store()
            .diagnostics(&version)
            .map(|diagnostics| diagnostics.as_ref().clone())
            .unwrap_or_default();

        Ok(diagnostics)
    }

    /// Return diagnostics for every recorded artifact in one revision.
    pub fn diagnostics(&self, revision: Revision) -> Result<DiagnosticCollection, RepositoryError> {
        let _revision = self.revision(revision)?;
        let mut diagnostics = DiagnosticCollection::new();

        for entry in self.artifact_versions.iter() {
            let ((entry_revision, _artifact_key), version) = entry.pair();
            if *entry_revision != revision {
                continue;
            }

            let artifact_diagnostics = self
                .artifact_store()
                .diagnostics(version)
                .map(|diagnostics| diagnostics.as_ref().clone())
                .unwrap_or_default();
            diagnostics.merge_from(&artifact_diagnostics);
        }

        Ok(diagnostics)
    }
}
