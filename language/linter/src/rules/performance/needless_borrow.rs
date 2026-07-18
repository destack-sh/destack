use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow borrows that add no required lifetime or capability.
    pub NEEDLESS_BORROW {
        id: "needless-borrow",
        code: "LP051",
        description: "Disallow borrows that add no required lifetime or capability",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check needless-borrow.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
