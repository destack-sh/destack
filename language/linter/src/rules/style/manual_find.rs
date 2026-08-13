use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer Array.find over a loop returning the first matching element.
    pub MANUAL_FIND {
        id: "manual-find",
        summary: "Prefer Array.find over a loop returning the first matching element",
        explanation: r#"
A loop that returns its first matching element and otherwise returns undefined spells Array.find manually.
Instead, you SHOULD return the result of `find` directly.
"#,
        example: {
            reported: r#"
function firstPositive(values: int32[]): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
            accepted: r#"
function firstPositive(values: int32[]): int32 | undefined {
    return values.find((value) => value > 0);
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Reject this lint until DIR carries checked flow uses.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} requires checked flow uses in DIR",
        lint.id
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Validate the canonical lint example.
    #[test]
    fn test_lint_example() {
        TestSession::assert_example(&MANUAL_FIND);
    }

    /// Replace a complete first-match loop.
    #[test]
    fn test_replaces_manual_find() {
        let session = TestSession::dir(&MANUAL_FIND, MANUAL_FIND.example.reported());

        session.assert_diagnostics(
            r#"
warning[manual-find]: loop manually finds its first matching value
 ──▶ main.ds:2:5
  │
1 │ function firstPositive(values: int32[]): int32 | undefined {
2 │     for (const value of values) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         if (value > 0) {
  │         ^^^^^^^^^^^^^^^^
4 │             return value;
  │             ^^^^^^^^^^^^^
5 │         }
  │         ^
6 │     }
  │     ^
7 │     return undefined;
8 │ }
  │

 = suggestion: return the first matching value directly (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function firstPositive(values: int32[]): int32 | undefined {
-   2│     for (const value of values) {
-   3│         if (value > 0) {
-   4│             return value;
-   5│         }
-   6│     }
-   7│     return undefined;
+   2│     return values.find((value) => value > 0);
"#,
        );
        session.assert_suggestions(MANUAL_FIND.example.accepted());
    }

    /// Accept returning a transformed matching value.
    #[test]
    fn test_accepts_transformed_return() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: int32[]): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value + 1;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a different fallback value.
    #[test]
    fn test_accepts_other_fallback() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: int32[]): int32 {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with an additional body action.
    #[test]
    fn test_accepts_additional_action() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: int32[], seen: int32[]): int32 | undefined {
    for (const value of values) {
        seen.push(value);
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept manual searching over a non-array iterable.
    #[test]
    fn test_accepts_other_iterable() {
        let session = TestSession::dir(
            &MANUAL_FIND,
            r#"
function firstPositive(values: Set<int32>): int32 | undefined {
    for (const value of values) {
        if (value > 0) {
            return value;
        }
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
