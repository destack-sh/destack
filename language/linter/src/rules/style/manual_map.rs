use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer Array.map over manually collecting transformed elements.
    pub MANUAL_MAP {
        id: "manual-map",
        summary: "Prefer Array.map over manually collecting transformed elements",
        explanation: r#"
Creating an array, pushing one transformed value for every input element, and returning it spells Array.map manually.
Instead, you SHOULD return the result of `map` directly.
"#,
        example: {
            reported: r#"
function doubled(values: int32[]): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        result.push(value * 2);
    }
    return result;
}
"#,
            accepted: r#"
function doubled(values: int32[]): int32[] {
    return values.map((value) => value * 2);
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
        TestSession::assert_example(&MANUAL_MAP);
    }

    /// Replace a complete push-and-return mapping loop.
    #[ignore]
    #[test]
    fn test_replaces_manual_map() {
        let session = TestSession::dir(&MANUAL_MAP, MANUAL_MAP.example.reported());

        session.assert_diagnostics(
            r#"
warning[manual-map]: loop manually collects mapped values
 ──▶ main.ds:3:5
  │
1 │ function doubled(values: int32[]): int32[] {
2 │     const result: int32[] = [];
3 │     for (const value of values) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
4 │         result.push(value * 2);
  │         ^^^^^^^^^^^^^^^^^^^^^^^
5 │     }
  │     ^
6 │     return result;
7 │ }
  │

 = suggestion: return the mapped array directly (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function doubled(values: int32[]): int32[] {
-   2│     const result: int32[] = [];
-   3│     for (const value of values) {
-   4│         result.push(value * 2);
-   5│     }
-   6│     return result;
+   2│     return values.map((value) => value * 2);
"#,
        );
        session.assert_suggestions(MANUAL_MAP.example.accepted());
    }

    /// Accept a loop with another body action.
    #[ignore]
    #[test]
    fn test_accepts_additional_action() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
function doubled(values: int32[]): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        result.push(value);
        result.push(value);
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
            &MANUAL_MAP,
            r#"
function copy(values: int32[]): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        result.push(value);
    }
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept collecting through a user-defined push method.
    #[ignore]
    #[test]
    fn test_accepts_user_push() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
class Values {
    push(value: int32): void {
        // intentionally empty
    }
}

function copy(source: int32[]): Values {
    const result = new Values();
    for (const value of source) {
        result.push(value);
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept manual mapping over a non-array iterable.
    #[ignore]
    #[test]
    fn test_accepts_other_iterable() {
        let session = TestSession::dir(
            &MANUAL_MAP,
            r#"
function doubled(values: Set<int32>): int32[] {
    const result: int32[] = [];
    for (const value of values) {
        result.push(value * 2);
    }
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
