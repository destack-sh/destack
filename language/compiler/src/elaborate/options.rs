use destack_repository::{CompilerOptions, Module, ProviderContext};

use crate::Compiler;

/// Options for one elaborate phase run.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ElaborateOptions {
    /// Whether to split multi-declarator lets into individual lets.
    pub(crate) split_declarators: bool,
    /// Whether to convert simple if else expressions to ternary form.
    pub(crate) ternary: bool,
    /// Whether to rewrite implicit expression returns to return statements.
    pub(crate) explicit_return: bool,
    /// Whether to wrap inserted implicit casts in parentheses.
    pub(crate) parenthesize_casts: bool,
}

impl Default for ElaborateOptions {
    /// Create default elaborate options.
    fn default() -> Self {
        Self {
            split_declarators: true,
            ternary: true,
            explicit_return: true,
            parenthesize_casts: false,
        }
    }
}

impl ElaborateOptions {
    /// Convert workspace compiler options into elaborate options.
    pub(crate) fn from_workspace(_options: &CompilerOptions) -> Self {
        Self::default()
    }
}

impl Compiler {
    /// Resolve elaborate options for one module.
    pub(crate) fn elaborate_options(
        &self,
        context: &dyn ProviderContext,
        module: &Module,
    ) -> ElaborateOptions {
        self.workspace_compiler_options(context, module)
            .map(|options| ElaborateOptions::from_workspace(&options))
            .unwrap_or_default()
    }
}
