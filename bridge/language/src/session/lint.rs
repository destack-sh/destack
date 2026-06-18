use crate::{Diagnostic, Module, PackageId, ProfileId, bridge};

/// Scope accepted by lint operations.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// One module profile.
    Module {
        /// Loaded source module.
        module: Module,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// One source package.
    Package {
        /// Source package.
        package: PackageId,
    },
    /// Whole workspace.
    Workspace,
}

/// One linter request.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintRequest {
    /// Scope to lint.
    pub scope: Scope,
}

/// One linter output.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintOutput {
    /// Diagnostics emitted by lint rules.
    pub diagnostics: Vec<Diagnostic>,
}
