use std::hash::Hash;

use destack_core::StringId;
use destack_source::{Diagnostic, DiagnosticCollection, ModuleId, PackageId, Span, TargetId};

use crate::{
    ArtifactDependency, ArtifactKey, ArtifactVersion, DiagnosticAnchor, DiagnosticError,
    DiagnosticSite, RequireError,
};

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

    /// Resolve one diagnostic anchor into a concrete source span.
    fn resolve_diagnostic_anchor(
        &self,
        anchor: &DiagnosticAnchor,
    ) -> Result<Option<Span>, DiagnosticError>;

    /// Resolve one provider diagnostic site into an exact anchor.
    fn anchor(&self, site: &DiagnosticSite) -> Result<DiagnosticAnchor, DiagnosticError>;

    /// Resolve one artifact key into an exact artifact version.
    fn artifact_version(&self, key: ArtifactKey) -> Result<ArtifactVersion, DiagnosticError>;

    /// Format one source string id when the context can resolve it.
    fn format_string_id(&self, string: StringId) -> Result<String, DiagnosticError>;

    /// Format one module id when the context can resolve it.
    fn format_module_id(&self, module: ModuleId) -> Result<String, DiagnosticError>;

    /// Format one package id when the context can resolve it.
    fn format_package_id(&self, package: PackageId) -> Result<String, DiagnosticError>;

    /// Format one target id when the context can resolve it.
    fn format_target_id(&self, target: TargetId) -> Result<String, DiagnosticError>;

    /// Add diagnostics produced by this attempt.
    fn diagnostics(&self, diagnostics: DiagnosticCollection);

    /// Add one finalized diagnostic produced by this attempt.
    fn diagnostic(&self, diagnostic: Diagnostic) {
        self.diagnostics(DiagnosticCollection::from_diagnostics(vec![diagnostic]));
    }
}
