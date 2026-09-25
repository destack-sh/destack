use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow nonterminal switch breaks inside an enclosing loop.
    pub INEFFECTIVE_BREAK_IN_SWITCH {
        id: "ineffective-break-in-switch",
        summary: "Disallow nonterminal switch breaks inside an enclosing loop",
        explanation: r#"
An unlabeled `break` inside a switch exits the switch even when the switch is nested in a loop.
Instead, you SHOULD label the enclosing loop when the break is intended to leave it.

The rule permits terminal case breaks used to prevent switch fallthrough.
"#,
        example: {
            reported: r#"
function visit(values: int32[]): void {
    outer: for (const value of values) {
        switch (value) {
            default:
                if (value < 0) {
                    break;
                }
                value;
        }
    }
}
"#,
            accepted: r#"
function visit(values: int32[]): void {
    outer: for (const value of values) {
        switch (value) {
            default:
                if (value < 0) {
                    break outer;
                }
                value;
        }
    }
}
"#,
        },
        provenance: [Unicorn("no-break-in-nested-loop")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report nonterminal switch breaks nested within another iteration.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect unlabeled valueless breaks whose target is a switch
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(
            node,
            dir::Expression::Break {
                label: None,
                value: None
            }
        ) {
            continue;
        }
        let target = module.transfer_target(expression)?;
        if !matches!(view.get(target), dir::Expression::Switch { .. }) {
            continue;
        }
        let Some(case) = view.ancestor::<dir::SwitchCase>(expression.into_any()) else {
            continue;
        };
        let case_body = view.get(case).body;
        if view.get(case_body).last_expression() == Some(expression)
            && view.get_parent_for(expression) == Some(case_body.into_any())
        {
            continue;
        }

        // require another iteration around the selected switch
        let mut parent = view.get_parent_any(target.into_any());
        let mut iteration = None;
        while let Some(node) = parent {
            if module.callable_body(node).is_some() {
                break;
            }
            if let Ok(expression) = node.try_into_typed::<dir::Expression>()
                && module.iteration_body(expression).is_some()
            {
                iteration = Some(expression);
                break;
            }

            parent = view.get_parent_any(node);
        }
        if iteration.is_none() {
            continue;
        }

        // report the break whose nearest target is the switch
        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic(
            "break exits the switch rather than the enclosing loop",
            span,
        ));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a conditional switch break inside a loop.
    #[test]
    fn test_reports_nonterminal_switch_break() {
        let session = TestSession::dir(
            &INEFFECTIVE_BREAK_IN_SWITCH,
            r#"
function visit(values: int32[]): void {
    outer: for (const value of values) {
        switch (value) {
            default:
                if (value < 0) {
                    break;
                }
                value;
        }
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[ineffective-break-in-switch]: break exits the switch rather than the enclosing loop
 ──▶ main.tspp:6:21
  │
4 │             default:
5 │                 if (value < 0) {
6 │                     break;
  │                     ^^^^^
7 │                 }
8 │                 value;
  │
"#,
        );
    }

    /// Accept a terminal switch break used to prevent fallthrough.
    #[test]
    fn test_accepts_terminal_case_break() {
        let session = TestSession::dir(
            &INEFFECTIVE_BREAK_IN_SWITCH,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        switch (value) {
            case 0:
                value;
                break;
            default:
                value;
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a conditional break even when its conditional ends the case.
    #[test]
    fn test_reports_terminal_conditional_break() {
        let session = TestSession::dir(
            &INEFFECTIVE_BREAK_IN_SWITCH,
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        switch (value) {
            default:
                if (value < 0) {
                    break;
                }
        }
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[ineffective-break-in-switch]: break exits the switch rather than the enclosing loop
 ──▶ main.tspp:6:21
  │
4 │             default:
5 │                 if (value < 0) {
6 │                     break;
  │                     ^^^^^
7 │                 }
8 │         }
  │
"#,
        );
    }

    /// Accept a labeled break that selects the enclosing loop.
    #[test]
    fn test_accepts_labeled_loop_break() {
        let session = TestSession::dir(
            &INEFFECTIVE_BREAK_IN_SWITCH,
            r#"
function visit(values: int32[]): void {
    outer: for (const value of values) {
        switch (value) {
            default:
                if (value < 0) {
                    break outer;
                }
                value;
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a switch break when no loop encloses the switch.
    #[test]
    fn test_accepts_switch_outside_loop() {
        let session = TestSession::dir(
            &INEFFECTIVE_BREAK_IN_SWITCH,
            r#"
function visit(value: int32): void {
    switch (value) {
        default:
            if (value < 0) {
                break;
            }
            value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
