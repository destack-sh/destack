use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
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
        provenance: [
            Clippy("never_loop"),
            Eslint("no-unreachable-loop"),
            SonarJs("no-one-iteration-loop"),
        ],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report loops whose flow cannot reach another iteration.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every authored iteration body
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(body) = module.iteration_body(expression) else {
            continue;
        };
        let block = view.get(body);
        let is_end_reachable = block
            .iter_expressions()
            .all(|expression| !module.flows.is_diverging(expression.into_any()));
        if is_end_reachable || module.has_reachable_continue(expression)? {
            continue;
        }

        // report the complete loop expression
        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint.diagnostic("loop cannot reach a second iteration", span);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a while loop that always breaks.
    #[test]
    fn test_reports_breaking_while() {
        let session = TestSession::dir(
            &LOOP_SINGLE_ITERATION,
            r#"
declare function ready(): boolean;
declare function process(): void;

while (ready()) {
    process();
    break;
}
"#,
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
  │     ^^^^^^^^^^
6 │     break;
  │     ^^^^^^
7 │ }
  │ ^
  │
"#,
        );
    }

    /// Report an unconditional loop that always returns.
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

    /// Report a stopped loop with a non-diverging unreachable body end.
    #[test]
    fn test_reports_loop_with_unreachable_body_end() {
        let session = TestSession::dir(
            &LOOP_SINGLE_ITERATION,
            r#"
function stop(): void {
    loop {
        return;
        0;
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
4 │         0;
  │         ^^
5 │     }
  │     ^
6 │ }
  │
"#,
        );
    }
}
