use destack_artifact::{ArtifactKey, ArtifactVersion};
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

    /// Record the exact artifact version for one revision-scoped key.
    pub fn record_artifact_version(
        &self,
        revision: Revision,
        version: ArtifactVersion,
    ) -> Result<(), RepositoryError> {
        let _revision = self.revision(revision)?;
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
}
