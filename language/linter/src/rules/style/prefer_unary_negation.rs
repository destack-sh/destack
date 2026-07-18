use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer unary negation over multiplying by -1.
    pub PREFER_UNARY_NEGATION {
        id: "prefer-unary-negation",
        code: "LY061",
        description: "Prefer unary negation over multiplying by -1",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-unary-negation.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
