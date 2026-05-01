use std::hash::Hash;

use destack_core::StringId;
use destack_source::{Diagnostic, DiagnosticCollection, ModuleId, PackageId, Span, TargetId};

use crate::{
    ArtifactDependency, ArtifactKey, ArtifactVersion, DiagnosticAnchor, DiagnosticError,
    DiagnosticSite, RequireError,
};

/// Context used to finalize provider diagnostics.
pub trait DiagnosticContext {
    /// The revision type used by the repository that owns this attempt.
    type Revision: Copy + Eq + Hash;

    /// Resolve one provider diagnostic site into a revision-stable anchor.
    fn anchor(&self, site: &DiagnosticSite) -> Result<DiagnosticAnchor, DiagnosticError>;

    /// Resolve one diagnostic anchor into a concrete source span.
    fn span(&self, anchor: &DiagnosticAnchor) -> Result<Span, DiagnosticError>;

    /// Resolve one artifact key into the artifact version visible to this attempt.
    fn artifact_version(&self, key: ArtifactKey) -> Result<ArtifactVersion, DiagnosticError>;

    /// Format one source string id relative to a diagnostic anchor.
    fn format_string_id(
        &self,
        anchor: &DiagnosticAnchor,
        string: StringId,
    ) -> Result<String, DiagnosticError>;

    /// Format one module id when the context can resolve it.
    fn format_module_id(&self, module: ModuleId) -> Result<String, DiagnosticError>;

    /// Format one package id when the context can resolve it.
    fn format_package_id(&self, package: PackageId) -> Result<String, DiagnosticError>;

    /// Format one target id when the context can resolve it.
    fn format_target_id(&self, target: TargetId) -> Result<String, DiagnosticError>;
}

/// Context exposed to one artifact provider attempt.
pub trait ProviderContext: DiagnosticContext {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Self::Revision;

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey;

    /// Require one artifact and return its exact version when ready.
    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, RequireError>;

    /// Add one exact dependency read by this attempt.
    fn depend_on(&self, dependency: ArtifactDependency);

    /// Add diagnostics produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: DiagnosticCollection);

    /// Add one finalized diagnostic produced by this attempt.
    fn emit_diagnostic(&self, diagnostic: Diagnostic) {
        self.emit_diagnostics(DiagnosticCollection::from_diagnostics(vec![diagnostic]));
    }
}
