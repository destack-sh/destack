use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Shorten the lifetime of values with significant destruction.
    pub SIGNIFICANT_DROP_TIGHTENING {
        id: "significant-drop-tightening",
        summary: "Shorten the lifetime of values with significant destruction",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check significant-drop-tightening.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
