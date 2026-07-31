use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow operator implementations built on a different operator.
    pub SUSPICIOUS_OPERATOR_IMPLEMENTATION {
        id: "suspicious-operator-implementation",
        summary: "Disallow operator implementations built on a different operator",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check suspicious-operator-implementation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
