use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow repeated string growth that causes cumulative copying.
    pub REPEATED_STRING_GROWTH {
        id: "repeated-string-growth",
        summary: "Disallow repeated string growth that causes cumulative copying",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check repeated-string-growth.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
