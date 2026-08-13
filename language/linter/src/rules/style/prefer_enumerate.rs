use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer Array.entries over manually counting array iteration.
    pub PREFER_ENUMERATE {
        id: "prefer-enumerate",
        summary: "Prefer Array.entries over manually counting array iteration",
        explanation: r#"
Initializing an index before array iteration and incrementing it after every element maintains information already provided by Array.entries.
Instead, you SHOULD bind each index and value from `entries`.
"#,
        example: {
            reported: r#"
function indexes(values: int32[], output: usize[]): void {
    let index: usize = 0;
    for (const value of values) {
        output.push(index);
        index++;
    }
}
"#,
            accepted: r#"
function indexes(values: int32[], output: usize[]): void {
    for (const (index, value) of values.entries()) {
        output.push(index);
    }
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
        TestSession::assert_example(&PREFER_ENUMERATE);
    }

    /// Replace a separately counted array loop.
    #[ignore]
    #[test]
    fn test_replaces_manual_index() {
        let session = TestSession::dir(&PREFER_ENUMERATE, PREFER_ENUMERATE.example.reported());

        session.assert_diagnostics(
            r#"
warning[prefer-enumerate]: array iteration maintains its index manually
 ──▶ main.ds:2:5
  │
1 │ function indexes(values: int32[], output: usize[]): void {
2 │     let index: usize = 0;
  │     ^^^^^^^^^^^^^^^^^^^^
3 │     for (const value of values) {
4 │         output.push(index);
  │

 = suggestion: iterate over array entries (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function indexes(values: int32[], output: usize[]): void {
-   2│     let index: usize = 0;
-   3│     for (const value of values) {
+   2│     for (const (index, value) of values.entries()) {

    4│         output.push(index);
-   5│         index++;
"#,
        );
        session.assert_suggestions(PREFER_ENUMERATE.example.accepted());
    }

    /// Report a counter used after the loop while withholding a rewrite.
    #[ignore]
    #[test]
    fn test_reports_counter_used_after_loop_without_suggestion() {
        let source = r#"
function count(values: int32[]): usize {
    let index: usize = 0;
    for (const value of values) {
        value;
        index;
        index++;
    }
    return index;
}
"#;
        let session = TestSession::dir(&PREFER_ENUMERATE, source);

        session.assert_diagnostics(
            r#"
warning[prefer-enumerate]: array iteration maintains its index manually
 ──▶ main.ds:2:5
  │
1 │ function count(values: int32[]): usize {
2 │     let index: usize = 0;
  │     ^^^^^^^^^^^^^^^^^^^^
3 │     for (const value of values) {
4 │         value;
  │
"#,
        );
    }

    /// Report a labeled loop while withholding a rewrite that would remove the label.
    #[ignore]
    #[test]
    fn test_reports_labeled_loop_without_suggestion() {
        let source = r#"
function indexes(values: int32[], output: usize[]): void {
    let index: usize = 0;
    outer: for (const value of values) {
        output.push(index);
        if (value < 0) {
            break outer;
        }
        index++;
    }
}
"#;
        let session = TestSession::dir(&PREFER_ENUMERATE, source);

        session.assert_diagnostics(
            r#"
warning[prefer-enumerate]: array iteration maintains its index manually
 ──▶ main.ds:2:5
  │
1 │ function indexes(values: int32[], output: usize[]): void {
2 │     let index: usize = 0;
  │     ^^^^^^^^^^^^^^^^^^^^
3 │     outer: for (const value of values) {
4 │         output.push(index);
  │
"#,
        );
    }

    /// Accept a counter that is not used during iteration.
    #[ignore]
    #[test]
    fn test_accepts_unused_counter() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function count(values: int32[]): usize {
    let index: usize = 0;
    for (const value of values) {
        value;
        index++;
    }
    return index;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept manually counting a non-array iterable.
    #[ignore]
    #[test]
    fn test_accepts_other_iterable() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function count(values: Set<int32>, output: usize[]): void {
    let index: usize = 0;
    for (const value of values) {
        output.push(index);
        value;
        index++;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a counter changed before its trailing increment.
    #[ignore]
    #[test]
    fn test_accepts_counter_mutation() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function indexes(values: int32[], output: usize[]): void {
    let index: usize = 0;
    for (const value of values) {
        index += 1;
        output.push(index);
        value;
        index++;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a counter whose increment can be skipped.
    #[ignore]
    #[test]
    fn test_accepts_skipped_increment() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function indexes(values: int32[], output: usize[]): void {
    let index: usize = 0;
    for (const value of values) {
        output.push(index);
        if (value < 0) {
            continue;
        }
        index++;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a counter captured by a nested lambda.
    #[ignore]
    #[test]
    fn test_accepts_captured_counter() {
        let session = TestSession::dir(
            &PREFER_ENUMERATE,
            r#"
function callbacks(values: int32[]): (() => usize)[] {
    const output: (() => usize)[] = [];
    let index: usize = 0;
    for (const value of values) {
        value;
        output.push(() => index);
        index++;
    }
    return output;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
