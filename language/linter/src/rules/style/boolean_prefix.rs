use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require predicate prefixes for boolean values.
    pub BOOLEAN_PREFIX {
        id: "boolean-prefix",
        summary: "Require predicate prefixes for boolean values",
        explanation: r#"
Boolean bindings, parameters, fields, and constants begin with `is`, `has`, `can`, `should`, `did`,
or `will` so their truth condition reads directly at use sites. Functions and methods are exempt
because predicate verbs such as `contains`, `matches`, and `startsWith` already express a boolean
result.
"#,
        example: {
            reported: r#"
struct Connection {
    active: boolean;
}

function send(ready: boolean): void;
"#,
            accepted: r#"
struct Connection {
    isActive: boolean;
}

function send(isReady: boolean): void;
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check boolean-prefix.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
