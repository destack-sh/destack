use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow useless string concatenation.
    pub NO_USELESS_CONCAT {
        id: "no-useless-concat",
        description: "Disallow useless string concatenation",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-concat.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
