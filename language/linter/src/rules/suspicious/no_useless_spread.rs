use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow spreads that have no observable effect.
    pub NO_USELESS_SPREAD {
        id: "no-useless-spread",
        description: "Disallow spreads that have no observable effect",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-spread.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
