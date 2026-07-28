use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow scrutinees that unnecessarily extend significant destruction.
    pub SIGNIFICANT_DROP_IN_SCRUTINEE {
        id: "significant-drop-in-scrutinee",
        summary: "Disallow scrutinees that unnecessarily extend significant destruction",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check significant-drop-in-scrutinee.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
