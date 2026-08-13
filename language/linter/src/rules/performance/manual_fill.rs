use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer fill over an index loop assigning one repeated value.
    pub MANUAL_FILL {
        id: "manual-fill",
        summary: "Prefer fill over an index loop assigning one repeated value",
        explanation: r#"
An index loop from zero to an array's length that assigns one repeated value spells the array fill operation manually.
Instead, you SHOULD call `fill` with that value.
"#,
        example: {
            reported: r#"
function clear(values: int32[]): void {
    for (let index: usize = 0; index < values.length; index++) {
        values[index] = 0;
    }
}
"#,
            accepted: r#"
function clear(values: int32[]): void {
    values.fill(0);
}
"#,
        },
        category: Performance,
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
    #[ignore]
    #[test]
    fn test_lint_example() {
        TestSession::assert_example(&MANUAL_FILL);
    }

    /// Replace a complete zero-to-length Array assignment loop.
    #[ignore]
    #[test]
    fn test_replaces_index_fill_loop() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clear(values: int32[]): void {
    for (let index: usize = 0; index < values.length; index++) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-fill]: index loop assigns one value to every element
 ──▶ main.ds:2:5
  │
1 │ function clear(values: int32[]): void {
2 │     for (let index: usize = 0; index < values.length; index++) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         values[index] = 0;
  │         ^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │

 = suggestion: fill the array directly (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function clear(values: int32[]): void {
-   2│     for (let index: usize = 0; index < values.length; index++) {
-   3│         values[index] = 0;
-   4│     }
+   2│     values.fill(0);
"#,
        );
        session.assert_suggestions(MANUAL_FILL.example.accepted());
    }

    /// Accept a loop that assigns a value derived from its counter.
    #[ignore]
    #[test]
    fn test_accepts_counter_value() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function indices(values: usize[]): void {
    for (let index: usize = 0; index < values.length; index++) {
        values[index] = index;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with a nonzero lower bound.
    #[ignore]
    #[test]
    fn test_accepts_partial_range() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function clearTail(values: int32[]): void {
    for (let index: usize = 1; index < values.length; index++) {
        values[index] = 0;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an effectful value evaluated once per iteration.
    #[ignore]
    #[test]
    fn test_accepts_effectful_value() {
        let session = TestSession::dir(
            &MANUAL_FILL,
            r#"
function fillRandom(values: float64[]): void {
    for (let index: usize = 0; index < values.length; index++) {
        values[index] = Math.random();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
