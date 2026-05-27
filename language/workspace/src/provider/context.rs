use destack_artifact::{
    ArtifactDependency, ArtifactKey, ArtifactSidecar, ArtifactVersion, DiagnosticContext,
    DiagnosticError, DiagnosticLike,
};
use destack_source::DiagnosticCollection;

use crate::Revision;

use super::ProviderError;

/// Context exposed to one artifact provider attempt.
pub trait ProviderContext: DiagnosticContext {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision;

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey;

    /// Require one artifact and return its exact version when ready.
    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError>;

    /// Require many artifacts and return their exact versions when ready.
    fn require_all(&self, keys: &[ArtifactKey]) -> Result<Vec<ArtifactVersion>, ProviderError>;

    /// Add one exact dependency read by this attempt.
    fn track(&self, dependency: ArtifactDependency);

    /// Add an already-final diagnostic collection produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: DiagnosticCollection);

    /// Add one sidecar produced by this attempt.
    fn emit_sidecar(&self, sidecar: ArtifactSidecar);

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError>;
}
