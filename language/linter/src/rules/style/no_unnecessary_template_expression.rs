use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow template expressions without formatting or conversion.
    pub NO_UNNECESSARY_TEMPLATE_EXPRESSION {
        id: "no-unnecessary-template-expression",
        summary: "Disallow template expressions without formatting or conversion",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-unnecessary-template-expression.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
