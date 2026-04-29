use std::hash::Hash;

use destack_source::DiagnosticCollection;

use crate::{ArtifactDependency, ArtifactKey, ArtifactPayload, ArtifactVersion, RequireError};

/// Context exposed to one artifact provider attempt.
pub trait ProviderContext {
    /// The revision type used by the repository that owns this attempt.
    type Revision: Copy + Eq + Hash;

    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Self::Revision;

    /// Return the artifact key being built.
    fn key(&self) -> ArtifactKey;

    /// Require one artifact and return its exact version when ready.
    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, RequireError>;

    /// Add one exact dependency read by this attempt.
    fn dependency(&self, dependency: ArtifactDependency);

    /// Add diagnostics produced by this attempt.
    fn diagnostics(&self, diagnostics: DiagnosticCollection);

    /// Set the typed payload produced by this attempt.
    fn payload(&self, payload: ArtifactPayload);
}
