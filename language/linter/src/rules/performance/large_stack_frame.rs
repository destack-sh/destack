use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow functions that require excessive stack storage.
    pub LARGE_STACK_FRAME {
        id: "large-stack-frame",
        summary: "Disallow functions that require excessive stack storage",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check large-stack-frame.
fn check(_module: &mut MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
