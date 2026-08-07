use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require canonical casing for declared identifiers.
    pub IDENTIFIER_CASE {
        id: "identifier-case",
        summary: "Require canonical casing for declared identifiers",
        explanation: r#"
Types, variants, extensions, and type parameters use PascalCase. Values, functions, labels, and
value parameters use camelCase. Imports, string-named members, foreign declarations, protocol
implementations, generated declarations, and language items retain their imposed names.
"#,
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
