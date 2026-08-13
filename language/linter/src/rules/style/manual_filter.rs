use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer Array.filter over manually collecting matching elements.
    pub MANUAL_FILTER {
        id: "manual-filter",
        summary: "Prefer Array.filter over manually collecting matching elements",
        explanation: r#"
Creating an array, conditionally pushing each input element, and returning it spells Array.filter manually.
Instead, you SHOULD return the result of `filter` directly.
"#,
        example: {
            reported: r#"
function positive(values: int32[]): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        }
    }
    return result;
}
"#,
            accepted: r#"
function positive(values: int32[]): int32[] {
    return values.filter((value) => value > 0);
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Validate the canonical lint example.
    #[ignore]
    #[test]
    fn test_lint_example() {
        TestSession::assert_example(&MANUAL_FILTER);
    }

    /// Replace a complete conditional push-and-return loop.
    #[ignore]
    #[test]
    fn test_replaces_manual_filter() {
        let session = TestSession::dir(&MANUAL_FILTER, MANUAL_FILTER.example.reported());

        session.assert_diagnostics(
            r#"
warning[manual-filter]: loop manually collects matching values
 ──▶ main.ds:3:5
  │
1 │ function positive(values: int32[]): int32[] {
2 │     const result: int32[] = [];
3 │     for (const value of values) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │         if (value > 0) {
  │         ^^^^^^^^^^^^^^^^
5 │             result.push(value);
  │             ^^^^^^^^^^^^^^^^^^^
6 │         }
  │         ^
7 │     }
  │     ^
8 │     return result;
9 │ }
  │

 = suggestion: return the filtered array directly (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function positive(values: int32[]): int32[] {
-   2│     const result: int32[] = [];
-   3│     for (const value of values) {
-   4│         if (value > 0) {
-   5│             result.push(value);
-   6│         }
-   7│     }
-   8│     return result;
+   2│     return values.filter((value) => value > 0);
"#,
        );
        session.assert_suggestions(MANUAL_FILTER.example.accepted());
    }

    /// Accept pushing a transformed element.
    #[ignore]
    #[test]
    fn test_accepts_transformed_push() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function positive(values: int32[]): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value + 1);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a conditional push with an else branch.
    #[ignore]
    #[test]
    fn test_accepts_else_branch() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function partition(values: int32[]): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        } else {
            result.push(0);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept returning a different array.
    #[ignore]
    #[test]
    fn test_accepts_different_return() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function positive(values: int32[]): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        }
    }
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept manual filtering over a non-array iterable.
    #[ignore]
    #[test]
    fn test_accepts_other_iterable() {
        let session = TestSession::dir(
            &MANUAL_FILTER,
            r#"
function positive(values: Set<int32>): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        if (value > 0) {
            result.push(value);
        }
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
