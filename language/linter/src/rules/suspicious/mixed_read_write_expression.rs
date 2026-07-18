use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow reading and mutating the same place within one larger expression.
    pub MIXED_READ_WRITE_EXPRESSION {
        id: "mixed-read-write-expression",
        code: "LU052",
        description: "Disallow reading and mutating the same place within one larger expression",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check mixed-read-write-expression.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
