use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow explicit lifetimes that elision already produces.
    pub NEEDLESS_LIFETIME_ANNOTATION {
        id: "needless-lifetime-annotation",
        summary: "Disallow explicit lifetimes that elision already produces",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check needless-lifetime-annotation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
