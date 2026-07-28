use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow reading and mutating the same place within one larger expression.
    pub MIXED_READ_WRITE_EXPRESSION {
        id: "mixed-read-write-expression",
        summary: "Disallow reading and mutating the same place within one larger expression",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check mixed-read-write-expression.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
