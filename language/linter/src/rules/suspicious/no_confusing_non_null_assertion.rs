use destack_ast::{self as ast, AssignOperator, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_is_optional_chain_target, expression_trailing_bang_span};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Disallow confusing non null assertions.
    ///
    /// This rule follows TS-ESLint `no-confusing-non-null-assertion` parity for
    /// confusing operator combinations like `a! == b`, `a! = b`, `a! in b`, and
    /// `a! instanceof b`.
    /// It also keeps Destack optional chain clarity checks as an enhancement.
    #[lint(
        id = "no-confusing-non-null-assertion",
        code = "LU005",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoConfusingNonNullAssertion,
    "Disallow confusing non null assertions"
}

impl LintRule for NoConfusingNonNullAssertion {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConfusingNonNullAssertion::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect all expressions for source parity and Destack enhancement cases
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // handle optional chain confusion enhancement
            if let Expression::Must { left, .. } = expression {
                if is_optional_chain_target(ctx, node_id) {
                    report_optional_chain_confusion_before(ctx, meta, node_id);
                }

                if is_optional_chain(ctx, *left) {
                    report_optional_chain_confusion_after(ctx, meta, node_id, *left);
                }
            }

            // handle TS-ESLint parity operator confusion cases
            let Some(operator_case) = confusing_operator_case(expression) else {
                continue;
            };
            if !has_confusing_non_null_left_operand(ctx, operator_case.left_expression_id) {
                continue;
            }
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let (message, label) = match operator_case.kind {
                ConfusingOperatorKind::Assign => (
                    "confusing combination of non-null assertion and assignment",
                    "`a! = b` can be mistaken for `a != b`",
                ),
                ConfusingOperatorKind::Equal => (
                    "confusing combination of non-null assertion and equality test",
                    "`a! == b` can be mistaken for `a !== b`",
                ),
                ConfusingOperatorKind::In => (
                    "confusing combination of non-null assertion and `in`",
                    "`a! in b` can be mistaken for `!(a in b)`",
                ),
                ConfusingOperatorKind::InstanceOf => (
                    "confusing combination of non-null assertion and `instanceof`",
                    "`a! instanceof b` can be mistaken for `!(a instanceof b)`",
                ),
            };

            let mut diagnostic = LintDiagnostic::new(
                NO_CONFUSING_NON_NULL_ASSERTION.id,
                NO_CONFUSING_NON_NULL_ASSERTION.code,
                NO_CONFUSING_NON_NULL_ASSERTION.category,
                severity,
                message,
                ctx.module.file_id,
                ctx.tree.get_span(node_id),
            )
            .with_label(label);

            if ctx.compute_fixes
                && let Some(fix) = confusing_operator_fix(
                    ctx,
                    operator_case.left_expression_id,
                    operator_case.kind,
                )
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// One source parity operator case for confusing non null syntax.
#[derive(Clone, Copy)]
struct ConfusingOperatorCase {
    /// The left operand expression id.
    left_expression_id: ast::LocalNodeId<Expression>,
    /// The matched operator kind.
    kind: ConfusingOperatorKind,
}

/// One confusing operator kind from source parity behavior.
#[derive(Clone, Copy)]
enum ConfusingOperatorKind {
    /// Assignment `=`.
    Assign,
    /// Equality `==` and `===`.
    Equal,
    /// Membership `in`.
    In,
    /// Instance check `instanceof`.
    InstanceOf,
}

/// Return one source parity operator case when one expression matches.
fn confusing_operator_case(expression: &Expression) -> Option<ConfusingOperatorCase> {
    match expression {
        Expression::Assign {
            left,
            operator: AssignOperator::Assign,
            ..
        } => Some(ConfusingOperatorCase {
            left_expression_id: *left,
            kind: ConfusingOperatorKind::Assign,
        }),
        Expression::Binary { left, operator, .. } => {
            let kind = match operator {
                BinaryOperator::Equal | BinaryOperator::EqualStrict => ConfusingOperatorKind::Equal,
                BinaryOperator::In => ConfusingOperatorKind::In,
                BinaryOperator::InstanceOf => ConfusingOperatorKind::InstanceOf,
                _ => return None,
            };

            Some(ConfusingOperatorCase {
                left_expression_id: *left,
                kind,
            })
        }
        _ => None,
    }
}

/// Return true when one left operand ends with a non null assertion and is not parenthesized.
fn has_confusing_non_null_left_operand(
    ctx: &LintAstContext<'_>,
    left_expression_id: ast::LocalNodeId<Expression>,
) -> bool {
    let left_expression = ctx.tree.get(left_expression_id);
    if matches!(left_expression, Expression::Parenthesized { .. }) {
        return false;
    }

    expression_trailing_bang_span(ctx, left_expression_id).is_some()
}

/// Build one fix for one source parity operator case.
fn confusing_operator_fix(
    ctx: &LintAstContext<'_>,
    left_expression_id: ast::LocalNodeId<Expression>,
    kind: ConfusingOperatorKind,
) -> Option<LintFix> {
    // remove the trailing `!` for assignment targets to keep syntax valid
    if matches!(kind, ConfusingOperatorKind::Assign) {
        let bang_span = expression_trailing_bang_span(ctx, left_expression_id)?;
        return Some(
            LintFix::suggestion("Remove non-null assertion from assignment target")
                .delete(bang_span),
        );
    }

    // parenthesize the full left operand for binary operators
    let left_span = ctx.tree.get_span(left_expression_id);
    let left_text = ctx.get_span_text(left_span);
    if left_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("({left_text})");
    let edits = ctx
        .edit_builder()
        .replace(left_span, replacement)
        .into_edits();
    Some(LintFix::suggestion("Wrap left operand in parentheses").with_edits(edits))
}

/// Report one optional chain confusion diagnostic for `foo!?.bar` forms.
fn report_optional_chain_confusion_before(
    ctx: &mut LintAstContext<'_>,
    meta: &'static crate::LintMeta,
    expression_id: ast::LocalNodeId<Expression>,
) {
    // resolve effective lint severity
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return;
    }

    // build one diagnostic for optional chain clarity
    let mut diagnostic = LintDiagnostic::new(
        NO_CONFUSING_NON_NULL_ASSERTION.id,
        NO_CONFUSING_NON_NULL_ASSERTION.code,
        NO_CONFUSING_NON_NULL_ASSERTION.category,
        severity,
        "confusing non-null assertion before optional chain",
        ctx.module.file_id,
        ctx.tree.get_span(expression_id),
    )
    .with_label("`!` before `?.` is confusing");

    // attach one safe grouping fix when requested
    if ctx.compute_fixes
        && let Some(fix) = parenthesize_non_null_before_optional_chain_fix(ctx, expression_id)
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Report one optional chain confusion diagnostic for `foo?.bar!` forms.
fn report_optional_chain_confusion_after(
    ctx: &mut LintAstContext<'_>,
    meta: &'static crate::LintMeta,
    must_expression_id: ast::LocalNodeId<Expression>,
    optional_chain_id: ast::LocalNodeId<Expression>,
) {
    // resolve effective lint severity
    let severity = ctx.get_effective_severity(meta, must_expression_id);
    if !severity.is_enabled() {
        return;
    }

    // build one diagnostic for optional chain clarity
    let mut diagnostic = LintDiagnostic::new(
        NO_CONFUSING_NON_NULL_ASSERTION.id,
        NO_CONFUSING_NON_NULL_ASSERTION.code,
        NO_CONFUSING_NON_NULL_ASSERTION.category,
        severity,
        "confusing non-null assertion after optional chain",
        ctx.module.file_id,
        ctx.tree.get_span(must_expression_id),
    )
    .with_label("`!` after `?.` chain is confusing");

    // attach one safe grouping fix when requested
    if ctx.compute_fixes
        && let Some(fix) = parenthesize_optional_chain_before_non_null_fix(
            ctx,
            must_expression_id,
            optional_chain_id,
        )
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Build one safe fix by parenthesizing the non null expression before optional chaining.
fn parenthesize_non_null_before_optional_chain_fix(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
) -> Option<LintFix> {
    let expression_span = ctx.tree.get_span(expression_id);
    let expression_text = ctx.get_span_text(expression_span);
    if expression_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("({expression_text})");
    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();
    Some(LintFix::safe("Add grouping around non-null assertion").with_edits(edits))
}

/// Build one safe fix by parenthesizing the optional chain before non null assertion.
fn parenthesize_optional_chain_before_non_null_fix(
    ctx: &LintAstContext<'_>,
    must_expression_id: ast::LocalNodeId<Expression>,
    optional_chain_id: ast::LocalNodeId<Expression>,
) -> Option<LintFix> {
    let optional_span = ctx.tree.get_span(optional_chain_id);
    let optional_text = ctx.get_span_text(optional_span);
    if optional_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("({optional_text})!");
    let must_span = ctx.tree.get_span(must_expression_id);
    let edits = ctx
        .edit_builder()
        .replace(must_span, replacement)
        .into_edits();
    Some(LintFix::safe("Add grouping around optional chain").with_edits(edits))
}

fn is_optional_chain_target(
    ctx: &LintAstContext<'_>,
    node_id: ast::LocalNodeId<Expression>,
) -> bool {
    expression_is_optional_chain_target(ctx.tree, ctx.parents, node_id)
}

/// Return true when one expression is or contains optional chaining.
fn is_optional_chain(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        Expression::Maybe { .. } => true,
        Expression::Member { left, .. } => is_optional_chain(ctx, *left),
        Expression::Index { left, .. } => is_optional_chain(ctx, *left),
        Expression::Call { left, .. } => is_optional_chain(ctx, *left),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_non_null_before_optional_chain() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_detects_non_null_before_optional_chain.ds",
            r#"
const x = foo!.?bar
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_detects_non_null_after_optional_chain() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_detects_non_null_after_optional_chain.ds",
            r#"
const x = foo?.bar!
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_detects_non_null_before_equality() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_detects_non_null_before_equality.ds",
            r#"
if (a! == b) {
    x()
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_detects_non_null_before_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_detects_non_null_before_assignment.ds",
            r#"
a! = b
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_detects_non_null_before_in_operator() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_detects_non_null_before_in_operator.ds",
            r#"
if (a! in b) {
    x()
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_allows_parenthesized_left_operand() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_allows_parenthesized_left_operand.ds",
            r#"
if ((a + b!) == c) {
    x()
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-non-null-assertion");
    }

    #[test]
    fn test_fix_wraps_left_operand_for_equality() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_fix_wraps_left_operand_for_equality.ds",
            r#"
if (a + b! == c) {
    x()
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion")
            .assert_suggested_fixed(
                r#"
if ((a + b!) == c) {
    x()
}
"#,
            );
    }

    #[test]
    fn test_fix_removes_non_null_for_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingNonNullAssertion);
        let result = test.lint_ast(
            "no_confusing_non_null_assertion/test_fix_removes_non_null_for_assignment.ds",
            r#"
a! = b
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-non-null-assertion")
            .assert_suggested_fixed(
                r#"
a = b;
"#,
            );
    }
}
