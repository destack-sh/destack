use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow materializing a collection consumed by one streaming operation.
    pub NEEDLESS_COLLECT {
        id: "needless-collect",
        code: "LP064",
        description: "Disallow materializing a collection consumed by one streaming operation",
        category: Performance,
        level: Warning,
        fixable: Sometimes,
        check: MirModule(check),
    }
}

/// Check needless-collect.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
