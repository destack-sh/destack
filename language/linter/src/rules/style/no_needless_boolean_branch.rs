use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow branches that only return boolean literals.
    pub NO_NEEDLESS_BOOLEAN_BRANCH {
        id: "no-needless-boolean-branch",
        summary: "Disallow branches that only return boolean literals",
        explanation: "An if statement whose two branches return opposite boolean literals only restates its condition. Return the condition directly, or negate it when the branches reverse the condition.",
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
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report if statements whose only branches return opposite boolean literals.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect regular if statements with one expression condition
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::If,
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
        let Some(then_value) = returned_boolean(view, *then_expression) else {
            continue;
        };
        let Some(else_value) = returned_boolean(view, *else_expression) else {
            continue;
        };
        if then_value == else_value {
            continue;
        }

        // report and replace both branches with one return
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("boolean branches only restate the condition", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            condition,
            !then_value && else_value,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the boolean literal returned by one single-statement branch.
fn returned_boolean(
    view: dir::View<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Option<bool> {
    let dir::Expression::Block(block) = view.get(expression) else {
        return None;
    };
    let expression = view.get(*block).only_expression()?;
    let dir::Expression::Return { value: Some(value) } = view.get(expression) else {
        return None;
    };

    view.get(*value).as_boolean()
}

/// Build an automatic return of the retained condition.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    condition: dir::LocalNodeId<dir::Expression>,
    is_negated: bool,
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
        module.negated_source(condition)?
    };
    let replacement = format!("return {condition};");
    let mut file = FilePatch::new(extent.file);
    file.replace(extent, replacement);
    let patches = PatchSet::single(file);
    let suggestion = lint.fix("return the boolean condition directly", patches)?;

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
warning[no-needless-boolean-branch]: boolean branches only restate the condition
 ──▶ main.ds:2:5
  │
1 │ function inactive(condition: boolean): boolean {
2 │     if (condition) {
  │     ^^^^^^^^^^^^^^^^
3 │         return false;
  │ ^^^^^^^^^^^^^^^^^^^^^
4 │     } else {
  │ ^^^^^^^^^^^^
5 │         return true;
  │ ^^^^^^^^^^^^^^^^^^^^
6 │     }
  │ ^^^^^
7 │ }
  │

 = fix: return the boolean condition directly
--- a/main.ds
+++ b/main.ds

    1│ function inactive(condition: boolean): boolean {
-   2│     if (condition) {
-   3│         return false;
-   4│     } else {
-   5│         return true;
-   6│     }
+   2│     return !condition;
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
}
