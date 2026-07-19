use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow useless computed keys.
    pub NO_USELESS_COMPUTED_KEY {
        id: "no-useless-computed-key",
        description: "Disallow useless computed keys",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-useless-computed-key.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
