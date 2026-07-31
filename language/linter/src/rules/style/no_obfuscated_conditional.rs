use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow conditional chains obscuring a simple branch.
    pub NO_OBFUSCATED_CONDITIONAL {
        id: "no-obfuscated-conditional",
        summary: "Disallow conditional chains obscuring a simple branch",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-obfuscated-conditional.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
