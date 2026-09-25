use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer expression initialization over staged assignments.
    pub PREFER_EXPRESSION_INITIALIZATION {
        id: "prefer-expression-initialization",
        summary: "Prefer expression initialization over staged assignments",
        explanation: r#"
A binding assigned immediately after its declaration is initialized in two separate places.
Instead, you SHOULD initialize the binding directly from the assigned expression.
"#,
        example: {
            reported: r#"
function select(condition: boolean): int32 {
    let result: int32;
    if (condition) {
        result = 1;
    } else {
        result = 2;
    }

    return result;
}
"#,
            accepted: r#"
function select(condition: boolean): int32 {
    const result = if (condition) {
        1
    } else {
        2
    };

    return result;
}
"#,
        },
        provenance: [Clippy("needless_late_init")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report adjacent uninitialized bindings and exhaustive branch assignments.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect adjacent expressions within every authored block
    for (_, block) in view.iter_nodes::<dir::Block>() {
        let expressions = block.iter_expressions().collect::<Vec<_>>();
        for pair in expressions.windows(2) {
            let [binding, initializer] = pair else {
                continue;
            };
            let Some(symbol) = uninitialized_binding(module, *binding)? else {
                continue;
            };
            if !initializes_binding(module, *initializer, symbol)? {
                continue;
            }

            // report the complete staged initialization
            let binding_span = module.source_extent(binding.into_any())?;
            let initializer_span = module.source_extent(initializer.into_any())?;
            let span = binding_span.merge(initializer_span);
            let diagnostic = lint
                .diagnostic(
                    "binding is assigned immediately after its declaration",
                    span,
                )
                .help("initialize the binding from the assigned expression");
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Return the symbol declared by one direct uninitialized `let` binding.
fn uninitialized_binding(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
    let view = module.view();
    let dir::Expression::Let {
        kind: dir::LetKind::Let,
        declarators,
        ..
    } = view.get(expression)
    else {
        return Ok(None);
    };
    let [declarator] = declarators.as_slice() else {
        return Ok(None);
    };
    let declarator = view.get(*declarator);
    if declarator.value.is_some()
        || !matches!(
            view.get(declarator.pattern),
            dir::Pattern::Binding { pattern: None, .. }
        )
    {
        return Ok(None);
    }

    module.declaration_symbol(declarator.pattern).map(Some)
}

/// Return whether one expression completely initializes the binding.
fn initializes_binding(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    symbol: dir::GlobalSymbolId,
) -> Result<bool, ProviderError> {
    let view = module.view();

    // accept one direct assignment
    if assigned_value(module, expression, symbol)?.is_some() {
        return Ok(true);
    }

    match view.get(expression) {
        // require both branches of an ordinary if statement
        dir::Expression::If {
            form: dir::IfForm::If,
            then_expression,
            else_expression: Some(else_expression),
            ..
        } => Ok(assigned_value(module, *then_expression, symbol)?.is_some()
            && assigned_value(module, *else_expression, symbol)?.is_some()),

        // require every arm of an exhaustive match
        dir::Expression::Match { arms, .. } if !arms.is_empty() => {
            for arm in arms {
                let body = match view.get(*arm) {
                    dir::MatchArm::Expression { body, .. } => *body,
                    dir::MatchArm::Block { body, .. } => {
                        let Some(body) = view.get(*body).only_expression() else {
                            return Ok(false);
                        };
                        body
                    }
                };
                if assigned_value(module, body, symbol)?.is_none() {
                    return Ok(false);
                }
            }

            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Return the value assigned to one symbol by a sole branch expression.
fn assigned_value(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    symbol: dir::GlobalSymbolId,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let view = module.view();
    let expression = match view.get(expression) {
        dir::Expression::Block(block) => {
            let Some(expression) = view.get(*block).only_expression() else {
                return Ok(None);
            };
            expression
        }
        _ => expression,
    };
    let Some(assignment) = module.place_assignment(expression) else {
        return Ok(None);
    };
    if assignment.operator != dir::AssignOperator::Assign
        || module.selected_symbol(assignment.target)? != Some(symbol)
    {
        return Ok(None);
    }

    Ok(Some(assignment.value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an adjacent direct assignment.
    #[test]
    fn test_reports_direct_initialization() {
        let session = TestSession::dir(
            &PREFER_EXPRESSION_INITIALIZATION,
            r#"
function select(): int32 {
    let result: int32;
    result = calculate();

    return result;
}
declare function calculate(): int32;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-expression-initialization]: binding is assigned immediately after its declaration
 ──▶ main.tspp:2:5
  │
1 │ function select(): int32 {
2 │     let result: int32;
  │     ^^^^^^^^^^^^^^^^^^
3 │     result = calculate();
  │     ^^^^^^^^^^^^^^^^^^^^
4 │
5 │     return result;
  │

 = help: initialize the binding from the assigned expression
"#,
        );
    }

    /// Report an adjacent binding assigned by both if branches.
    #[test]
    fn test_reports_if_branch_initialization() {
        let session = TestSession::dir(
            &PREFER_EXPRESSION_INITIALIZATION,
            r#"
function select(condition: boolean): int32 {
    let result: int32;
    if (condition) {
        result = 1;
    } else {
        result = 2;
    }

    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-expression-initialization]: binding is assigned immediately after its declaration
 ──▶ main.tspp:2:5
  │
1 │ function select(condition: boolean): int32 {
2 │     let result: int32;
  │     ^^^^^^^^^^^^^^^^^^
3 │     if (condition) {
  │     ^^^^^^^^^^^^^^^^
4 │         result = 1;
  │         ^^^^^^^^^^^
5 │     } else {
  │     ^^^^^^^^
6 │         result = 2;
  │         ^^^^^^^^^^^
7 │     }
  │     ^
8 │
9 │     return result;
  │

 = help: initialize the binding from the assigned expression
"#,
        );
    }

    /// Report an adjacent binding assigned by every match arm.
    #[test]
    fn test_reports_match_arm_initialization() {
        let session = TestSession::dir(
            &PREFER_EXPRESSION_INITIALIZATION,
            r#"
function describe(value: int32): string {
    let result: string;
    match (value) {
        0 => result = "zero"
        _ => result = "other"
    }

    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-expression-initialization]: binding is assigned immediately after its declaration
 ──▶ main.tspp:2:5
  │
1 │ function describe(value: int32): string {
2 │     let result: string;
  │     ^^^^^^^^^^^^^^^^^^^
3 │     match (value) {
  │     ^^^^^^^^^^^^^^^
4 │         0 => result = "zero"
  │         ^^^^^^^^^^^^^^^^^^^^
5 │         _ => result = "other"
  │         ^^^^^^^^^^^^^^^^^^^^^
6 │     }
  │     ^
7 │
8 │     return result;
  │

 = help: initialize the binding from the assigned expression
"#,
        );
    }

    /// Accept branches that perform work besides the assignment.
    #[test]
    fn test_accepts_branch_with_additional_work() {
        let session = TestSession::dir(
            &PREFER_EXPRESSION_INITIALIZATION,
            r#"
function select(condition: boolean): int32 {
    let result: int32;
    if (condition) {
        result = 1;
        record(result);
    } else {
        result = 2;
    }

    return result;
}
declare function record(value: int32): void;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept nonadjacent staged assignments.
    #[test]
    fn test_accepts_intervening_expression() {
        let session = TestSession::dir(
            &PREFER_EXPRESSION_INITIALIZATION,
            r#"
function select(condition: boolean): int32 {
    let result: int32;
    record();
    if (condition) {
        result = 1;
    } else {
        result = 2;
    }

    return result;
}
declare function record(): void;
"#,
        );

        session.assert_no_diagnostics();
    }
}
