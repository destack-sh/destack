use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require both halves of the iteration protocol.
    pub ITERATOR_PROTOCOL_COMPLETENESS {
        id: "iterator-protocol-completeness",
        summary: "Require both halves of the iteration protocol",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check iterator-protocol-completeness.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
