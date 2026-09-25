use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow conditionals that only produce opposite boolean literals.
    pub NO_NEEDLESS_BOOLEAN_BRANCH {
        id: "no-needless-boolean-branch",
        summary: "Disallow conditionals that only produce opposite boolean literals",
        explanation: r#"
An `if` or ternary whose branches produce opposite boolean literals has the same result as its condition or its negation.
Instead, you SHOULD use the condition directly or negate it when the branches reverse the result.
"#,
        example: {
            reported: r#"
function active(condition: boolean): boolean {
    if (condition) {
        return true;
    } else {
        return false;
    }
}
"#,
            accepted: r#"
function active(condition: boolean): boolean {
    return condition;
}
"#,
        },
        provenance: [
            Clippy("needless_bool"),
            Eslint("no-unneeded-ternary"),
        ],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report conditionals whose only branches produce opposite boolean literals.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect complete conditionals with one expression condition
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form,
            condition,
            then_expression,
            else_expression: Some(else_expression),
        } = node
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };

        // read the direct ternary values or the values returned by statement branches
        let (then_value, else_value) = match form {
            dir::IfForm::If => (
                module
                    .sole_return_value(*then_expression)
                    .and_then(|value| view.get(value).as_boolean()),
                module
                    .sole_return_value(*else_expression)
                    .and_then(|value| view.get(value).as_boolean()),
            ),
            dir::IfForm::Ternary => (
                view.get(*then_expression).as_boolean(),
                view.get(*else_expression).as_boolean(),
            ),
        };
        let Some(then_value) = then_value else {
            continue;
        };
        let Some(else_value) = else_value else {
            continue;
        };
        if then_value == else_value {
            continue;
        }

        // report and replace both branches with the retained condition
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("conditional only restates its condition", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            condition,
            !then_value && else_value,
            *form,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build an automatic replacement from the retained condition.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    condition: dir::LocalNodeId<dir::Expression>,
    is_negated: bool,
    form: dir::IfForm,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let condition_span = module.source_extent(condition.into_any())?;

    // do not discard comments outside the retained condition
    if module.has_unretained_comment(extent, &[condition_span])? {
        return Ok(None);
    }

    // retain or negate the exact authored condition
    let condition = if !is_negated {
        module.source(condition_span)?.to_string()
    } else {
        let condition = module.expression_source(condition, dir::OperatorPrecedence::Prefix)?;

        format!("!{condition}")
    };
    let replacement = match form {
        dir::IfForm::If => format!("return {condition};"),
        dir::IfForm::Ternary => condition,
    };
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.fix("use the boolean condition directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace reversed return branches with a negated condition.
    #[test]
    fn test_replaces_reversed_boolean_returns() {
        let session = TestSession::dir(
            &NO_NEEDLESS_BOOLEAN_BRANCH,
            r#"
function inactive(condition: boolean): boolean {
    if (condition) {
        return false;
    } else {
        return true;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-needless-boolean-branch]: conditional only restates its condition
 ──▶ main.tspp:2:5
  │
1 │ function inactive(condition: boolean): boolean {
2 │     if (condition) {
  │     ^^^^^^^^^^^^^^^^
3 │         return false;
  │         ^^^^^^^^^^^^^
4 │     } else {
  │     ^^^^^^^^
5 │         return true;
  │         ^^^^^^^^^^^^
6 │     }
  │     ^
7 │ }
  │

 = fix: use the boolean condition directly
--- a/main.tspp
+++ b/main.tspp

    1│ function inactive(condition: boolean): boolean {
-   2│     if (condition) {
-   3│         return false;
-   4│     } else {
-   5│         return true;
-   6│     }
+   2│     return !condition;
    7│ }
"#,
        );
        session.assert_fixes(
            r#"
function inactive(condition: boolean): boolean {
    return !condition;
}
"#,
        );
    }

    /// Accept branches that perform additional work before returning.
    #[test]
    fn test_accepts_nontrivial_boolean_branches() {
        let session = TestSession::dir(
            &NO_NEEDLESS_BOOLEAN_BRANCH,
            r#"
declare function record(): void;
function active(condition: boolean): boolean {
    if (condition) {
        record();
        return true;
    } else {
        return false;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept branches that return the same boolean literal.
    #[test]
    fn test_accepts_equal_boolean_returns() {
        let session = TestSession::dir(
            &NO_NEEDLESS_BOOLEAN_BRANCH,
            r#"
function active(condition: boolean): boolean {
    if (condition) {
        return true;
    } else {
        return true;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace reversed ternary branches with a negated condition.
    #[test]
    fn test_replaces_reversed_boolean_ternary() {
        let session = TestSession::dir(
            &NO_NEEDLESS_BOOLEAN_BRANCH,
            r#"
function inactive(condition: boolean): boolean {
    return condition ? false : true;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-needless-boolean-branch]: conditional only restates its condition
 ──▶ main.tspp:2:12
  │
1 │ function inactive(condition: boolean): boolean {
2 │     return condition ? false : true;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the boolean condition directly
--- a/main.tspp
+++ b/main.tspp

    1│ function inactive(condition: boolean): boolean {
-   2│     return condition ? false : true;
+   2│     return !condition;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
function inactive(condition: boolean): boolean {
    return !condition;
}
"#,
        );
    }

    /// Preserve precedence when negating a compound ternary condition.
    #[test]
    fn test_parenthesizes_negated_compound_ternary_condition() {
        let session = TestSession::dir(
            &NO_NEEDLESS_BOOLEAN_BRANCH,
            r#"
function inactive(left: boolean, right: boolean): boolean {
    return left && right ? false : true;
}
"#,
        );

        session.assert_fixes(
            r#"
function inactive(left: boolean, right: boolean): boolean {
    return !(left && right);
}
"#,
        );
    }

    /// Accept ternaries that select non-boolean values.
    #[test]
    fn test_accepts_value_ternary() {
        let session = TestSession::dir(
            &NO_NEEDLESS_BOOLEAN_BRANCH,
            r#"
function select(condition: boolean): int32 {
    return condition ? 1 : 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
