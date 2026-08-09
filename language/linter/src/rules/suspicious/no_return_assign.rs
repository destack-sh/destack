use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assignment within an explicit or implicit return value.
    pub NO_RETURN_ASSIGN {
        id: "no-return-assign",
        summary: "Disallow assignment within an explicit or implicit return value",
        explanation: r#"
Returning an assignment combines mutation with the function result and can be mistaken for an
equality test. Perform the assignment as a statement, then return the resulting value explicitly.
"#,
        example: {
            reported: r#"
function reset(value: int32): int32 {
    let current = value;
    return current = 0;
}
"#,
            accepted: r#"
function reset(value: int32): int32 {
    let current = value;
    current = 0;
    return current;
}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report assignments evaluated as part of return values.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect assignments enclosed by one explicit or implicit return
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !matches!(node, dir::Expression::Assign { .. }) {
            continue;
        }

        // report only the outermost assignment expression
        if matches!(
            view.ancestor::<dir::Expression>(expression.into_any())
                .map(|parent| view.get(parent)),
            Some(dir::Expression::Assign { .. })
        ) {
            continue;
        }

        // require an explicit or implicit return position
        if !is_returned(&view, expression) {
            continue;
        }

        // report the returned mutation
        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic("return value is an assignment", span));
    }

    Ok(output)
}

/// Return whether one assignment belongs to an explicit or implicit return.
fn is_returned(view: &dir::View<'_>, assignment: dir::LocalNodeId<dir::Expression>) -> bool {
    let mut node = assignment.into_any();

    // climb to the enclosing return or function boundary
    while let Some(parent) = view.get_parent_any(node) {
        if let Ok(expression) = parent.try_into_typed::<dir::Expression>() {
            if matches!(view.get(expression), dir::Expression::Return { .. }) {
                return true;
            }
        } else if let Ok(declaration) = parent.try_into_typed::<dir::Declaration>() {
            let dir::Declaration::Function(function) = view.get(declaration) else {
                return false;
            };

            return function
                .body
                .is_some_and(|body| !matches!(view.get(body), dir::Expression::Block(_)));
        } else if let Ok(member) = parent.try_into_typed::<dir::Member>() {
            let dir::Member::Method {
                body: Some(body), ..
            } = view.get(member)
            else {
                return false;
            };

            return !matches!(view.get(*body), dir::Expression::Block(_));
        } else if let Ok(property) = parent.try_into_typed::<dir::Property>() {
            if let dir::Property::Method {
                body: Some(body), ..
            } = view.get(property)
            {
                return !matches!(view.get(*body), dir::Expression::Block(_));
            }
        }

        node = parent;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept an assignment followed by a separate return.
    #[test]
    fn test_accepts_separate_assignment() {
        let session = TestSession::dir(
            &NO_RETURN_ASSIGN,
            r#"
function reset(value: int32): int32 {
    let current = value;
    current = 0;
    return current;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report an assignment returned by an expression-bodied lambda.
    #[test]
    fn test_reports_implicit_assignment() {
        let session = TestSession::dir(
            &NO_RETURN_ASSIGN,
            r#"
let current = 1;
const reset: () => int32 = (): int32 => current = 0;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-return-assign]: return value is an assignment
 ──▶ main.ds:2:41
  │
1 │ let current = 1;
2 │ const reset: () => int32 = (): int32 => current = 0;
  │                                         ^^^^^^^^^^^
  │
"#,
        );
    }

    /// Report an assignment nested within an explicit return expression.
    #[test]
    fn test_reports_nested_return_assignment() {
        let session = TestSession::dir(
            &NO_RETURN_ASSIGN,
            r#"
function reset(isActive: boolean): int32 {
    let current: int32 = 1;
    return isActive ? current = 0 : current;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-return-assign]: return value is an assignment
 ──▶ main.ds:3:23
  │
1 │ function reset(isActive: boolean): int32 {
2 │     let current: int32 = 1;
3 │     return isActive ? current = 0 : current;
  │                       ^^^^^^^^^^^
4 │ }
  │
"#,
        );
    }
}
