use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require grouping where operator precedence is easy to misread.
    pub AMBIGUOUS_PRECEDENCE {
        id: "ambiguous-precedence",
        code: "LU001",
        description: "Require grouping where operator precedence is easy to misread",
        category: Suspicious,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check ambiguous-precedence.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
