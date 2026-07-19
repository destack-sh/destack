use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirProgram, Lint, LintResult};

declare_lint! {
    /// Warn on exports unreachable from every target consumer.
    pub UNREACHABLE_EXPORT {
        id: "unreachable-export",
        description: "Warn on exports unreachable from every target consumer",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirProgram(check),
    }
}

/// Check unreachable-export.
fn check(_program: &DirProgram, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
