use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer startsWith or endsWith over equivalent string comparisons.
    pub PREFER_STRING_STARTS_ENDS_WITH {
        id: "prefer-string-starts-ends-with",
        summary: "Prefer startsWith or endsWith over equivalent string comparisons",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-string-starts-ends-with.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
