use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Warn when argument names indicate swapped positional arguments.
    pub SWAPPED_ARGUMENTS {
        id: "swapped-arguments",
        code: "LC046",
        description: "Warn when argument names indicate swapped positional arguments",
        category: Correctness,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check swapped-arguments.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
