use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow void expressions that accomplish nothing.
    pub NO_MEANINGLESS_VOID {
        id: "no-meaningless-void",
        code: "LU049",
        description: "Disallow void expressions that accomplish nothing",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-meaningless-void.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
