use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::{FileId, ModuleId, PackageId, Span};

/// Provider-side source anchor for one diagnostic label.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum DiagnosticAnchor {
    /// A concrete span in the provider revision.
    Span(Span),
    /// A whole source file in the provider revision.
    File(FileId),
    /// A module in the provider revision.
    Module(ModuleId),
    /// A package in the provider revision.
    Package(PackageId),
}

impl From<Span> for DiagnosticAnchor {
    /// Create a diagnostic anchor from a source span.
    fn from(span: Span) -> Self {
        Self::Span(span)
    }
}

impl From<FileId> for DiagnosticAnchor {
    /// Create a diagnostic anchor from a source file.
    fn from(file: FileId) -> Self {
        Self::File(file)
    }
}

impl From<ModuleId> for DiagnosticAnchor {
    /// Create a diagnostic anchor from a source module.
    fn from(module: ModuleId) -> Self {
        Self::Module(module)
    }
}

impl From<PackageId> for DiagnosticAnchor {
    /// Create a diagnostic anchor from a source package.
    fn from(package: PackageId) -> Self {
        Self::Package(package)
    }
}
