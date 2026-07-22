use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow constructors that only repeat implicit construction behavior.
    pub NO_USELESS_CONSTRUCTOR {
        id: "no-useless-constructor",
        summary: "Disallow constructors that only repeat implicit construction behavior",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-useless-constructor.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
