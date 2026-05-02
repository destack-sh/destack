use destack_source::{DiagnosticCollection, DiagnosticLabel, ModuleId, PackageId, TargetId};

use crate::{
    ArtifactDependency, ArtifactKey, ArtifactVersion, DiagnosticAnchor, DiagnosticError,
    DiagnosticLike, RequireError,
};

/// Context used to finalize provider diagnostics.
pub trait DiagnosticContext {
    /// Resolve one provider diagnostic anchor into a final source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError>;

    /// Format one module id when the context can resolve it.
    fn format_module_id(&self, module: ModuleId) -> Result<String, DiagnosticError>;

    /// Format one package id when the context can resolve it.
    fn format_package_id(&self, package: PackageId) -> Result<String, DiagnosticError>;

    /// Format one target id when the context can resolve it.
    fn format_target_id(&self, target: TargetId) -> Result<String, DiagnosticError>;
}

/// Context exposed to one artifact provider attempt.
pub trait ProviderContext: DiagnosticContext {
    /// The revision type used by the repository that owns this attempt.
    type Revision: Copy + Eq;

    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Self::Revision;

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey;

    /// Require one artifact and return its exact version when ready.
    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, RequireError>;

    /// Add one exact dependency read by this attempt.
    fn track(&self, dependency: ArtifactDependency);

    /// Add an already-final diagnostic collection produced by this attempt.
    fn emit_collection(&self, diagnostics: DiagnosticCollection);

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError>;
}
