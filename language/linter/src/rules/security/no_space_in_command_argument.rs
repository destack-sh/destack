use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow process arguments that embed spaces.
    pub NO_SPACE_IN_COMMAND_ARGUMENT {
        id: "no-space-in-command-argument",
        summary: "Disallow process arguments that embed spaces",
        category: Security,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-space-in-command-argument.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
