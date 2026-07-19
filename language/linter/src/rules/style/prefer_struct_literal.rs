use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer struct literal form.
    pub PREFER_STRUCT_LITERAL {
        id: "prefer-struct-literal",
        description: "Prefer struct literal form",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check prefer-struct-literal.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
