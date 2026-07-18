use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when repeated string growth causes cumulative copying.
    pub REPEATED_STRING_GROWTH {
        id: "repeated-string-growth",
        code: "LP060",
        description: "Warn when repeated string growth causes cumulative copying",
        category: Performance,
        level: Warning,
        fixable: Never,
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
