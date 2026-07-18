use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow template expressions without formatting or conversion.
    pub NO_UNNECESSARY_TEMPLATE_EXPRESSION {
        id: "no-unnecessary-template-expression",
        code: "LY068",
        description: "Disallow template expressions without formatting or conversion",
        category: Style,
        level: Warning,
        fixable: Never,
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
