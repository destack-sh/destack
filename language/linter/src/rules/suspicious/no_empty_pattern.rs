use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow empty destructuring patterns.
    pub NO_EMPTY_PATTERN {
        id: "no-empty-pattern",
        summary: "Disallow empty destructuring patterns",
        explanation: r#"
An empty destructuring pattern reads a value without selecting any of its fields or elements.
Instead, you SHOULD bind the intended values or remove the ineffective destructuring operation.
"#,
        example: {
            reported: r#"
function read(value: { name: string }): void {
    const {} = value;
}
"#,
            accepted: r#"
function read(value: { name: string }): void {
    const { name } = value;
}
"#,
        },
        provenance: [Eslint("no-empty-pattern")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report empty patterns used for destructuring rather than matching.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect patterns used only for destructuring
    for (pattern, node) in view.iter_nodes::<dir::Pattern>() {
        let is_empty = matches!(
            node,
            dir::Pattern::Object { fields }
                | dir::Pattern::Sequence { fields }
                | dir::Pattern::Tuple { fields }
                if fields.is_empty()
        );
        if !is_empty || is_match_context(&view, pattern) {
            continue;
        }

        let span = module.source_extent(pattern.into_any())?;
        output.report(lint.diagnostic("destructuring pattern selects no values", span));
    }

    // inspect destructuring assignment targets
    for (pattern, node) in view.iter_nodes::<dir::AssignPattern>() {
        let is_empty = matches!(
            node,
            dir::AssignPattern::Object { fields }
                | dir::AssignPattern::Sequence { fields }
                | dir::AssignPattern::Tuple { fields }
                if fields.is_empty()
        );
        if !is_empty {
            continue;
        }

        let span = module.source_extent(pattern.into_any())?;
        output.report(lint.diagnostic("destructuring pattern selects no values", span));
    }

    Ok(output)
}

/// Return whether one pattern participates in an explicit value match.
fn is_match_context(view: &dir::View<'_>, pattern: dir::LocalNodeId<dir::Pattern>) -> bool {
    // accept patterns within the selected pattern of a match arm
    if let Some(arm) = view.ancestor::<dir::MatchArm>(pattern.into_any()) {
        let arm_pattern = view.get(arm).pattern();
        if view.is_inside(pattern.into_any(), arm_pattern.into_any()) {
            return true;
        }
    }

    // select the declarator that owns a conditional pattern
    let Some(declarator) = view.ancestor::<dir::Declarator>(pattern.into_any()) else {
        return false;
    };
    let Some(parent) = view.get_parent_for(declarator) else {
        return false;
    };

    // recognize condition and let-else pattern positions
    match parent.ty {
        dir::NodeType::Expression => {
            let expression = view.get(dir::LocalNodeId::<dir::Expression>::new(parent.id));

            matches!(
                expression,
                dir::Expression::LetElse { declarator: binding, .. }
                    if *binding == declarator
            ) || matches!(
                expression,
                dir::Expression::If { condition, .. }
                    | dir::Expression::While { condition, .. }
                    if condition.binds(declarator)
            )
        }
        dir::NodeType::MatchArm => {
            let arm = view.get(dir::LocalNodeId::<dir::MatchArm>::new(parent.id));

            arm.guard()
                .is_some_and(|condition| condition.binds(declarator))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept an empty sequence used as a value-matching pattern.
    #[test]
    fn test_accepts_empty_match_pattern() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function isEmpty(values: int32[]): boolean {
    return match (values) {
        [] => true
        _ => false
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an empty sequence used as a conditional value match.
    #[test]
    fn test_accepts_empty_condition_pattern() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function isEmpty(values: int32[]): boolean {
    if (let [] = values) {
        return true;
    }
    return false;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an empty sequence used as a while-loop value match.
    #[test]
    fn test_accepts_empty_while_condition_pattern() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function waitUntilEmpty(values: int32[]): void {
    while (let [] = values) {
        return;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an empty pattern used by a match guard condition.
    #[test]
    fn test_accepts_empty_match_guard_pattern() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function isEmpty(value: () | null): boolean {
    return match (value) {
        tuple if (let () = tuple) => true
        _ => false
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an empty sequence used as a let-else value match.
    #[test]
    fn test_accepts_empty_let_else_pattern() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function acceptEmpty(values: int32[]): void {
    let [] = values else {
        return;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a destructuring pattern with an elision position.
    #[test]
    fn test_accepts_nonempty_sequence_pattern() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function skipFirst(values: int32[]): void {
    const [, second] = values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report an empty pattern nested beneath a named field.
    #[test]
    fn test_reports_nested_empty_pattern() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function discard(value: { user: { name: string } }): void {
    const { user: {} } = value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty-pattern]: destructuring pattern selects no values
 ──▶ main.tspp:2:19
  │
1 │ function discard(value: { user: { name: string } }): void {
2 │     const { user: {} } = value;
  │                   ^^
3 │ }
  │
"#,
        );
    }

    /// Report empty destructuring inside a match arm body.
    #[test]
    fn test_reports_empty_pattern_inside_match_arm() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function discardInsideArm(value: { name: string }): void {
    match (value) {
        _ => {
            const {} = value;
        }
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty-pattern]: destructuring pattern selects no values
 ──▶ main.tspp:4:19
  │
2 │     match (value) {
3 │         _ => {
4 │             const {} = value;
  │                   ^^
5 │         }
6 │     }
  │
"#,
        );
    }

    /// Report an empty destructuring assignment.
    #[test]
    fn test_reports_empty_assignment_pattern() {
        let session = TestSession::dir(
            &NO_EMPTY_PATTERN,
            r#"
function discard(value: { name: string }): void {
    ({} = value);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty-pattern]: destructuring pattern selects no values
 ──▶ main.tspp:2:6
  │
1 │ function discard(value: { name: string }): void {
2 │     ({} = value);
  │      ^^
3 │ }
  │
"#,
        );
    }
}
