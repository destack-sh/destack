use destack_dir as dir;
use destack_source::{FilePatch, PatchSet};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow negation in equality checks.
    pub NO_NEGATION_IN_EQUALITY_CHECK {
        id: "no-negation-in-equality-check",
        summary: "Disallow negation in equality checks",
        explanation: r#"
`!left === right` negates only the left operand, although its visual shape can be read as negating the complete equality check.
Instead, you SHOULD parenthesize the intended grouping or use the opposite equality operator.
"#,
        example: {
            reported: r#"
function differs(left: boolean, right: boolean): boolean {
    return !left === right;
}
"#,
            accepted: r#"
function differs(left: boolean, right: boolean): boolean {
    return !(left === right);
}
"#,
        },
        provenance: [Unicorn("no-negation-in-equality-check")],
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report equality checks whose left operand alone is negated.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect equality expressions
    for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = expression
        else {
            continue;
        };
        if !operator.is_equality() {
            continue;
        }
        let Some(resolution) = module.operator_decision(expression_id.into_any())? else {
            continue;
        };
        if !resolution.is_builtin() {
            continue;
        }
        let dir::Expression::Unary {
            operator: dir::UnaryOperator::Not,
            right: value,
        } = view.get(*left)
        else {
            continue;
        };

        // report the ambiguous authored negation
        let negation_span = module.main_span(left.into_any())?;
        let mut diagnostic = lint.diagnostic("left equality operand is negated", negation_span);

        // suggest regrouping only when it preserves boolean equality
        let Some(value_operand) = module.builtin_operand(left.into_any(), *value)? else {
            continue;
        };
        let Some(right_operand) = module.builtin_operand(expression_id.into_any(), *right)? else {
            continue;
        };
        let can_regroup = value_operand
            .scalar_families
            .as_ref()
            .is_some_and(|families| families.is_only_domain(dir::ScalarDomain::Boolean))
            && right_operand
                .scalar_families
                .as_ref()
                .is_some_and(|families| families.is_only_domain(dir::ScalarDomain::Boolean));
        if can_regroup {
            let comparison_span = module.source_extent(expression_id.into_any())?;
            let mut file_patch = FilePatch::new(comparison_span.file);
            file_patch.replace(negation_span, "!(");
            file_patch.insert(comparison_span.end, ")");
            file_patch.sort();
            let patches = PatchSet::from_files(vec![file_patch]);
            let suggestion = lint.suggestion("negate the complete equality check", patches)?;
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Preserve comments while regrouping the negation.
    #[test]
    fn test_preserves_equality_comments() {
        let session = TestSession::dir(
            &NO_NEGATION_IN_EQUALITY_CHECK,
            r#"
function differs(left: boolean, right: boolean): boolean {
    return !(/* left */ left) === /* right */ right;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-negation-in-equality-check]: left equality operand is negated
 ──▶ main.ds:2:12
  │
1 │ function differs(left: boolean, right: boolean): boolean {
2 │     return !(/* left */ left) === /* right */ right;
  │            ^
3 │ }
  │

 = suggestion: negate the complete equality check (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function differs(left: boolean, right: boolean): boolean {
-   2│     return !(/* left */ left) === /* right */ right;
+   2│     return !((/* left */ left) === /* right */ right);
    3│ }
"#,
        );
        session.assert_suggestions(
            r#"
function differs(left: boolean, right: boolean): boolean {
    return !((/* left */ left) === /* right */ right);
}
"#,
        );
    }

    /// Report negation on the left of strict inequality.
    #[test]
    fn test_reports_negated_left_inequality_operand() {
        let session = TestSession::dir(
            &NO_NEGATION_IN_EQUALITY_CHECK,
            r#"
function same(left: boolean, right: boolean): boolean {
    return !left !== right;
}
"#,
        );

        session.assert_suggestions(
            r#"
function same(left: boolean, right: boolean): boolean {
    return !(left !== right);
}
"#,
        );
    }

    /// Accept a negated right operand because its grouping is unambiguous.
    #[test]
    fn test_accepts_negated_right_equality_operand() {
        let session = TestSession::dir(
            &NO_NEGATION_IN_EQUALITY_CHECK,
            r#"
function same(left: boolean, right: boolean): boolean {
    return left === !right;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report truthiness negation without suggesting a behavior change.
    #[test]
    fn test_reports_non_boolean_negation_without_suggestion() {
        let session = TestSession::dir(
            &NO_NEGATION_IN_EQUALITY_CHECK,
            r#"
function hasValue(value: string): boolean {
    return !value === false;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-negation-in-equality-check]: left equality operand is negated
 ──▶ main.ds:2:12
  │
1 │ function hasValue(value: string): boolean {
2 │     return !value === false;
  │            ^
3 │ }
  │
"#,
        );
    }
}
