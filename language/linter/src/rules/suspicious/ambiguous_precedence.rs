use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require grouping where operator precedence is easy to misread.
    pub AMBIGUOUS_PRECEDENCE {
        id: "ambiguous-precedence",
        summary: "Require grouping where operator precedence is easy to misread",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check ambiguous-precedence.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
