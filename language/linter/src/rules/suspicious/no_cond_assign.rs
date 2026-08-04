use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assignment in conditions.
    pub NO_COND_ASSIGN {
        id: "no-cond-assign",
        summary: "Disallow assignment in conditions",
        explanation: "An assignment used as a condition is easily mistaken for a comparison and hides mutation inside control flow. Move the assignment before the condition, or use a binding condition when the assigned value is intentionally tested.",
        example: {
            reported: r#"
function select(next: boolean): boolean {
    let active = false;
    if (active = next) {
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
    for expression in view.iter_nodes::<dir::Expression>() {
        match view.get(expression) {
            dir::Expression::If { condition, .. } => {
                for operand in &condition.operands {
                    if let dir::ConditionOperand::Expression { condition } = operand {
                        conditions.insert(condition.into_any());
                    }
                }
            }
            dir::Expression::While { condition, .. } => {
                conditions.insert(condition.into_any());
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
    for arm in view.iter_nodes::<dir::MatchArm>() {
        if let Some(guard) = view.get(arm).guard() {
            conditions.insert(guard.into_any());
        }
    }

    // inspect assignments evaluated inside those conditions
    for expression in view.iter_nodes::<dir::Expression>() {
        if !matches!(view.get(expression), dir::Expression::Assign { .. })
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
 ──▶ main.ds:3:12
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

    /// Report an assignment in a three-part for-loop condition.
    #[test]
    fn test_reports_for_condition_assignment() {
        let session = TestSession::dir(
            &NO_COND_ASSIGN,
            r#"
function repeat(next: boolean): void {
    let active = false;
    for (; active = next;) {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-cond-assign]: assignment is evaluated as a condition
 ──▶ main.ds:3:12
  │
1 │ function repeat(next: boolean): void {
2 │     let active = false;
3 │     for (; active = next;) {}
  │            ^^^^^^^^^^^^^
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
        _ if (active = next) => active
        _ => false
    };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-cond-assign]: assignment is evaluated as a condition
 ──▶ main.ds:4:15
  │
2 │     let active = false;
3 │     return match (value) {
4 │         _ if (active = next) => active
  │               ^^^^^^^^^^^^^
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
