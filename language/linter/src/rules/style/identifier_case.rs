use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require canonical casing for declared identifiers.
    pub IDENTIFIER_CASE {
        id: "identifier-case",
        summary: "Require canonical casing for declared identifiers",
        explanation: "Types and type parameters use PascalCase, ordinary values and callables use camelCase, and exported platform constants use UPPER_SNAKE_CASE. Names imposed by foreign interfaces, protocol implementations, generated declarations, and language items are exempt.",
        example: {
            reported: r#"
struct user_record {
    display_name: string;
}

function Load_User(): user_record;
"#,
            accepted: r#"
struct UserRecord {
    displayName: string;
}

function loadUser(): UserRecord;
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check identifier-case.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
