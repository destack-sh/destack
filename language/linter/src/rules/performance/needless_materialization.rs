use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow collections built only for immediate consumption.
    pub NEEDLESS_MATERIALIZATION {
        id: "needless-materialization",
        code: "LP061",
        description: "Disallow collections built only for immediate consumption",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check needless-materialization.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
