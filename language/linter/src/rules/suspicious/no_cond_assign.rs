use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assignment in conditions.
    pub NO_COND_ASSIGN {
        id: "no-cond-assign",
        summary: "Disallow assignment in conditions",
        explanation: r#"
An assignment expression used as a condition tests the assigned value after mutating its target, which can be mistaken for an equality comparison.
Instead, you SHOULD move the assignment before the condition or use a binding condition when the assigned value is intentionally tested.
"#,
        example: {
            reported: r#"
function select(next: boolean): boolean {
    let active = false;
    if ((active = next)) {
        return active;
    }
    return false;
}
"#,
            accepted: r#"
function select(next: boolean): boolean {
    let active = next;
    if (active) {
        return active;
    }
    return false;
}
"#,
        },
        provenance: [Eslint("no-cond-assign")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report assignment expressions evaluated by control-flow conditions.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut conditions = FxIndexSet::default();
    let mut output = LintOutput::default();

    // collect every authored expression condition
    for (_, expression) in view.iter_nodes::<dir::Expression>() {
        match expression {
            dir::Expression::If { condition, .. } | dir::Expression::While { condition, .. } => {
                for condition in condition.expressions() {
                    conditions.insert(condition.into_any());
                }
            }
            dir::Expression::For {
                condition: Some(condition),
                ..
            } => {
                conditions.insert(condition.into_any());
            }
            _ => {}
        }
    }

    // collect match guard expressions
    for (_, arm) in view.iter_nodes::<dir::MatchArm>() {
        if let Some(guard) = arm.guard() {
            for condition in guard.expressions() {
                conditions.insert(condition.into_any());
            }
        }
    }

    // inspect assignments evaluated inside those conditions
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Assign { .. })
            || !is_condition_assignment(view, expression.into_any(), &conditions)
        {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("assignment is evaluated as a condition", span)
            .help("move the assignment before the condition or use a binding condition");
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one assignment is evaluated by an enclosing condition.
fn is_condition_assignment(
    view: dir::View<'_>,
    assignment: dir::LocalNodeIdAny,
    conditions: &FxIndexSet<dir::LocalNodeIdAny>,
) -> bool {
    let mut node = assignment;

    // climb the evaluated expression until its condition or a nested declaration
    loop {
        if conditions.contains(&node) {
            return true;
        }
        let Some(parent) = view.get_parent_any(node) else {
            return false;
        };
        if let Ok(parent) = parent.try_into_typed::<dir::Expression>()
            && matches!(view.get(parent), dir::Expression::Declaration(_))
        {
            return false;
        }

        node = parent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a parenthesized assignment in a while condition.
    #[test]
    fn test_reports_parenthesized_while_assignment() {
        let session = TestSession::dir(
            &NO_COND_ASSIGN,
            r#"
function repeat(next: boolean): void {
    let active = false;
    while ((active = next)) {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-cond-assign]: assignment is evaluated as a condition
 ──▶ main.tspp:3:12
  │
1 │ function repeat(next: boolean): void {
2 │     let active = false;
3 │     while ((active = next)) {}
  │            ^^^^^^^^^^^^^^^
4 │ }
  │

 = help: move the assignment before the condition or use a binding condition
"#,
        );
    }

    /// Report an assignment after a while-loop condition binding.
    #[test]
    fn test_reports_while_condition_binding_assignment() {
        let session = TestSession::dir(
            &NO_COND_ASSIGN,
            r#"
function repeat(value: boolean | undefined): void {
    let active = false;
    while (let current! = value && (active = current)) {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-cond-assign]: assignment is evaluated as a condition
 ──▶ main.tspp:3:36
  │
1 │ function repeat(value: boolean | undefined): void {
2 │     let active = false;
3 │     while (let current! = value && (active = current)) {}
  │                                    ^^^^^^^^^^^^^^^^^^
4 │ }
  │

 = help: move the assignment before the condition or use a binding condition
"#,
        );
    }

    /// Report an assignment in a three-part for-loop condition.
    #[test]
    fn test_reports_for_condition_assignment() {
        let session = TestSession::dir(
            &NO_COND_ASSIGN,
            r#"
function repeat(next: boolean): void {
    let active = false;
    for (; (active = next); ) {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-cond-assign]: assignment is evaluated as a condition
 ──▶ main.tspp:3:12
  │
1 │ function repeat(next: boolean): void {
2 │     let active = false;
3 │     for (; (active = next); ) {}
  │            ^^^^^^^^^^^^^^^
4 │ }
  │

 = help: move the assignment before the condition or use a binding condition
"#,
        );
    }

    /// Report an assignment in a match-arm guard.
    #[test]
    fn test_reports_match_guard_assignment() {
        let session = TestSession::dir(
            &NO_COND_ASSIGN,
            r#"
function select(value: int32, next: boolean): boolean {
    let active = false;
    return match (value) {
        _ if ((active = next)) => active
        _ => false
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-cond-assign]: assignment is evaluated as a condition
 ──▶ main.tspp:4:15
  │
2 │     let active = false;
3 │     return match (value) {
4 │         _ if ((active = next)) => active
  │               ^^^^^^^^^^^^^^^
5 │         _ => false
6 │     };
  │

 = help: move the assignment before the condition or use a binding condition
"#,
        );
    }

    /// Report an assignment after a match-guard binding.
    #[test]
    fn test_reports_match_binding_guard_assignment() {
        let session = TestSession::dir(
            &NO_COND_ASSIGN,
            r#"
function select(value: (int32, boolean) | null, next: boolean): boolean {
    let active = false;
    return match (value) {
        pair if (let (number, ready) = pair && (active = next)) => ready
        _ => false
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-cond-assign]: assignment is evaluated as a condition
 ──▶ main.tspp:4:48
  │
2 │     let active = false;
3 │     return match (value) {
4 │         pair if (let (number, ready) = pair && (active = next)) => ready
  │                                                ^^^^^^^^^^^^^^^
5 │         _ => false
6 │     };
  │

 = help: move the assignment before the condition or use a binding condition
"#,
        );
    }

    /// Accept assignment before a condition.
    #[test]
    fn test_accepts_assignment_before_condition() {
        let session = TestSession::dir(
            &NO_COND_ASSIGN,
            r#"
function select(next: boolean): boolean {
    let active = false;
    active = next;
    if (active) {
        return active;
    }
    return false;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
