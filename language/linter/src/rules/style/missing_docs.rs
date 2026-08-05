use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require documentation for meaningful named declarations.
    pub MISSING_DOCS {
        id: "missing-docs",
        summary: "Require documentation for meaningful named declarations",
        explanation: "Functions, methods, types, fields, variants, and constants require concise documentation whether or not they are exported. Parameters, local bindings, generated declarations, and obvious protocol implementations are exempt; documentation does not require argument, error, or panic sections.",
        example: {
            reported: r#"
struct Session {
    userId: UserId;
}
"#,
            accepted: r#"
/// One authenticated user session.
struct Session {
    /// The authenticated user.
    userId: UserId;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check missing-docs.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
