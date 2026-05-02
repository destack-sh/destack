use destack_source::{DiagnosticLabel, ModuleId, PackageId, TargetId};

use crate::{DiagnosticAnchor, DiagnosticError};

/// Repository-backed value displayed in diagnostic text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticDisplay {
    /// Display one module.
    Module(ModuleId),
    /// Display one package.
    Package(PackageId),
    /// Display one target.
    Target(TargetId),
}

/// Context used to finalize provider diagnostics.
pub trait DiagnosticContext {
    /// Resolve one provider diagnostic anchor into a final source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError>;

    /// Display one repository-backed value when the context can resolve it.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError>;
}
