use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow unnecessary ternary expressions.
    pub NO_UNNEEDED_TERNARY {
        id: "no-unneeded-ternary",
        summary: "Disallow unnecessary ternary expressions",
        explanation: "A ternary that chooses opposite boolean literals only restates its boolean condition. Use the condition directly, or negate it when the branches reverse the condition.",
        example: {
            reported: r#"
function active(condition: boolean): boolean {
    return condition ? true : false;
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

/// Report ternaries that only select opposite boolean literals.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect ternaries with one expression condition and two boolean branches
    for expression in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::Ternary,
            condition,
            then_expression,
            else_expression: Some(else_expression),
        } = view.get(expression)
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };
        let Some(then_value) = view.get(*then_expression).as_boolean() else {
            continue;
        };
        let Some(else_value) = view.get(*else_expression).as_boolean() else {
            continue;
        };
        if then_value == else_value {
            continue;
        }

        // report and retain the exact condition expression
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("ternary only restates its condition", span);
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

/// Build an automatic replacement from the retained condition.
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
    let replacement = if !is_negated {
        module.source(condition_span)?.to_string()
    } else {
        module.negated_source(condition)?
    };
    let mut file = FilePatch::new(extent.file);
    file.replace(extent, replacement);
    let patches = PatchSet::single(file);
    let suggestion = lint.fix("use the boolean condition directly", patches)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace reversed boolean branches with a negated condition.
    #[test]
    fn test_replaces_reversed_boolean_ternary() {
        let session = TestSession::dir(
            &NO_UNNEEDED_TERNARY,
            r#"
function inactive(condition: boolean): boolean {
    return condition ? false : true;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-unneeded-ternary]: ternary only restates its condition
 ──▶ main.ds:2:12
  │
1 │ function inactive(condition: boolean): boolean {
2 │     return condition ? false : true;
  │            ^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │

 = fix: use the boolean condition directly
--- a/main.ds
+++ b/main.ds

    1│ function inactive(condition: boolean): boolean {
-   2│     return condition ? false : true;
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

    /// Preserve precedence when negating a compound condition.
    #[test]
    fn test_parenthesizes_negated_compound_condition() {
        let session = TestSession::dir(
            &NO_UNNEEDED_TERNARY,
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
            &NO_UNNEEDED_TERNARY,
            r#"
function select(condition: boolean): int32 {
    return condition ? 1 : 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
