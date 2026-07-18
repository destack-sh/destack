use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow fallbacks that cannot affect a spread.
    pub NO_USELESS_SPREAD_FALLBACK {
        id: "no-useless-spread-fallback",
        code: "LU046",
        description: "Disallow fallbacks that cannot affect a spread",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-spread-fallback.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
