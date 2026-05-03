use destack_workspace::{CompilerOptions, DiagnosticPolicy};

/// Default integer width for unannotated integer types.
pub(crate) const DEFAULT_INT_WIDTH: u16 = 32;

/// Default float width for unannotated float types.
pub(crate) const DEFAULT_FLOAT_WIDTH: u16 = 64;

/// Options used while importing source into declared DIR.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ImportOptions {
    /// Policy for local redeclarations in Destack modules.
    pub no_redeclared_locals: DiagnosticPolicy,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self::from_workspace(&CompilerOptions::default())
    }
}

impl ImportOptions {
    /// Build import options from repository compiler configuration.
    pub(crate) fn from_workspace(options: &CompilerOptions) -> Self {
        Self {
            no_redeclared_locals: options.no_redeclared_locals,
        }
    }
}
