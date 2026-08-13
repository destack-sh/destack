use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow loops whose control flow cannot reach another iteration.
    pub LOOP_SINGLE_ITERATION {
        id: "loop-single-iteration",
        summary: "Disallow loops whose control flow cannot reach another iteration",
        explanation: r#"
A loop whose body always exits or diverges cannot reach a second iteration and obscures its actual control flow.
Instead, you SHOULD use a conditional for optional execution or state an unconditional transfer directly.
"#,
        example: {
            reported: r#"
declare function ready(): boolean;
declare function process(): void;

while (ready()) {
    process();
    break;
}
"#,
            accepted: r#"
declare function ready(): boolean;
declare function process(): void;

if (ready()) {
    process();
}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: None,
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
        TestSession::assert_example(&LOOP_SINGLE_ITERATION);
    }

    /// Report a while loop that always breaks.
    #[ignore]
    #[test]
    fn test_reports_breaking_while() {
        let session = TestSession::dir(
            &LOOP_SINGLE_ITERATION,
            LOOP_SINGLE_ITERATION.example.reported(),
        );

        session.assert_diagnostics(
            r#"
warning[loop-single-iteration]: loop cannot reach a second iteration
 ──▶ main.ds:4:1
  │
2 │ declare function process(): void;
3 │
4 │ while (ready()) {
  │ ^^^^^^^^^^^^^^^^^
5 │     process();
  │ ^^^^^^^^^^^^^^^^^
6 │     break;
  │     ^^^^^^
7 │ }
  │ ^
  │
"#,
        );
    }

    /// Report an unconditional loop that always returns.
    #[ignore]
    #[test]
    fn test_reports_returning_loop() {
        let session = TestSession::dir(
            &LOOP_SINGLE_ITERATION,
            r#"
function first(): int32 {
    loop {
        return 1;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[loop-single-iteration]: loop cannot reach a second iteration
 ──▶ main.ds:2:5
  │
1 │ function first(): int32 {
2 │     loop {
  │     ^^^^^^
3 │         return 1;
  │         ^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │
"#,
        );
    }

    /// Accept a loop whose body can complete normally.
    #[ignore]
    #[test]
    fn test_accepts_repeating_loop() {
        let session = TestSession::dir(
            &LOOP_SINGLE_ITERATION,
            r#"
declare function ready(): boolean;
declare function process(): void;

while (ready()) {
    process();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop whose terminal continue selects itself.
    #[ignore]
    #[test]
    fn test_accepts_continuing_loop() {
        let session = TestSession::dir(
            &LOOP_SINGLE_ITERATION,
            r#"
loop {
    continue;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report an inner loop that continues only its outer loop.
    #[ignore]
    #[test]
    fn test_reports_outer_continue_from_inner_loop() {
        let session = TestSession::dir(
            &LOOP_SINGLE_ITERATION,
            r#"
outer: loop {
    loop {
        continue outer;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[loop-single-iteration]: loop cannot reach a second iteration
 ──▶ main.ds:2:5
  │
1 │ outer: loop {
2 │     loop {
  │     ^^^^^^
3 │         continue outer;
  │         ^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │
"#,
        );
    }

    /// Ignore an unreachable continue when deciding whether the loop can repeat.
    #[ignore]
    #[test]
    fn test_ignores_unreachable_continue() {
        let session = TestSession::dir(
            &LOOP_SINGLE_ITERATION,
            r#"
function stop(): void {
    loop {
        return;
        continue;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[loop-single-iteration]: loop cannot reach a second iteration
 ──▶ main.ds:2:5
  │
1 │ function stop(): void {
2 │     loop {
  │     ^^^^^^
3 │         return;
  │         ^^^^^^^
4 │         continue;
  │         ^^^^^^^^^
5 │     }
  │     ^
6 │ }
  │
"#,
        );
    }
}
