use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow variants that disproportionately enlarge an inline union.
    pub LARGE_VARIANT {
        id: "large-variant",
        summary: "Disallow variants that disproportionately enlarge an inline union",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check large-variant.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
