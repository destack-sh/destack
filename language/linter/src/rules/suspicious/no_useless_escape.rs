use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow escape sequences that do not change the parsed value.
    pub NO_USELESS_ESCAPE {
        id: "no-useless-escape",
        code: "LU039",
        description: "Disallow escape sequences that do not change the parsed value",
        category: Suspicious,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-useless-escape.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
