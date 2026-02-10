use destack_ast::{self as ast, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer range `in` operator over comparison chains.
    ///
    /// Checking if a value is within a range using comparison chains like
    /// `x >= start && x < end` can be more clearly expressed using the
    /// range membership operator `x in start..end`.
    ///
    /// ```
    /// // bad
    /// if x >= 0 && x < 10 { }
    /// if x > 0 && x <= 10 { }
    ///
    /// // good
    /// if x in 0..10 { }
    /// if x in 1..=10 { }
    /// ```
    #[lint(
        id = "prefer-range-contains",
        code = "LY051",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferRangeContains,
    "Prefer range in operator over comparison chains"
}

impl LintRule for PreferRangeContains {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferRangeContains::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // look for && expressions
            let Expression::Binary {
                operator: BinaryOperator::And,
                left,
                right,
            } = expression
            else {
                continue;
            };

            let Some(range_check) = extract_range_check(ctx, *left, *right) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                PREFER_RANGE_CONTAINS.id,
                PREFER_RANGE_CONTAINS.code,
                PREFER_RANGE_CONTAINS.category,
                severity,
                "use `in` operator with range instead of comparison chain",
                ctx.module.file_id,
                ctx.tree.get_span(node_id),
            )
            .with_label("replace with `x in start..end`");

            // rewrite comparable chains where lower bound is inclusive
            if ctx.compute_fixes
                && let Some(fix) = build_range_contains_fix(ctx, node_id, &range_check)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// One comparison bound descriptor.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BoundKind {
    /// Lower bound, either `>` or `>=`.
    Lower { is_inclusive: bool },
    /// Upper bound, either `<` or `<=`.
    Upper { is_inclusive: bool },
}

/// One normalized comparison expression.
struct ComparisonInfo {
    /// The compared variable name.
    variable_name: ast::StringId,
    /// The compared variable expression.
    variable_id: ast::LocalNodeId<Expression>,
    /// The bound expression on the right side.
    bound_id: ast::LocalNodeId<Expression>,
    /// The bound kind.
    bound_kind: BoundKind,
}

/// One normalized range check extracted from two comparisons.
struct RangeCheckInfo {
    /// The variable expression being tested.
    variable_id: ast::LocalNodeId<Expression>,
    /// The lower bound expression.
    lower_bound_id: ast::LocalNodeId<Expression>,
    /// The upper bound expression.
    upper_bound_id: ast::LocalNodeId<Expression>,
    /// Whether the lower bound is inclusive.
    lower_inclusive: bool,
    /// Whether the upper bound is inclusive.
    upper_inclusive: bool,
}

/// Extract a range check from a binary `&&` chain.
fn extract_range_check(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<Expression>,
    right_id: ast::LocalNodeId<Expression>,
) -> Option<RangeCheckInfo> {
    // get comparison info from both sides
    let left_cmp = get_comparison_info(ctx, left_id);
    let right_cmp = get_comparison_info(ctx, right_id);

    let (Some(left_info), Some(right_info)) = (left_cmp, right_cmp) else {
        return None;
    };

    // check if they're comparing the same variable
    if left_info.variable_name != right_info.variable_name {
        return None;
    }

    match (left_info.bound_kind, right_info.bound_kind) {
        (
            BoundKind::Lower {
                is_inclusive: lower_inclusive,
            },
            BoundKind::Upper {
                is_inclusive: upper_inclusive,
            },
        ) => Some(RangeCheckInfo {
            variable_id: left_info.variable_id,
            lower_bound_id: left_info.bound_id,
            upper_bound_id: right_info.bound_id,
            lower_inclusive,
            upper_inclusive,
        }),
        (
            BoundKind::Upper {
                is_inclusive: upper_inclusive,
            },
            BoundKind::Lower {
                is_inclusive: lower_inclusive,
            },
        ) => Some(RangeCheckInfo {
            variable_id: left_info.variable_id,
            lower_bound_id: right_info.bound_id,
            upper_bound_id: left_info.bound_id,
            lower_inclusive,
            upper_inclusive,
        }),
        _ => None,
    }
}

/// Extract comparison info from a binary comparison expression.
fn get_comparison_info(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
) -> Option<ComparisonInfo> {
    let expression = ctx.tree.get(expression_id);
    let Expression::Binary {
        operator,
        left,
        right,
    } = expression
    else {
        return None;
    };

    let left_expression = ctx.tree.get(*left);
    let Expression::Path {
        path,
        static_arguments: None,
    } = left_expression
    else {
        return None;
    };
    if path.segments.len() != 1 {
        return None;
    }

    let variable_name = path.segments[0];
    let variable_id = *left;
    let bound_id = *right;

    // determine the comparison kind and which side has the variable
    let bound_kind = match operator {
        BinaryOperator::LessThan => BoundKind::Upper {
            is_inclusive: false,
        },
        BinaryOperator::LessThanOrEqual => BoundKind::Upper { is_inclusive: true },
        BinaryOperator::GreaterThan => BoundKind::Lower {
            is_inclusive: false,
        },
        BinaryOperator::GreaterThanOrEqual => BoundKind::Lower { is_inclusive: true },
        _ => return None,
    };

    Some(ComparisonInfo {
        variable_name,
        variable_id,
        bound_id,
        bound_kind,
    })
}

/// Build an unsafe replacement when a range check maps directly to `in`.
fn build_range_contains_fix(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
    range_check: &RangeCheckInfo,
) -> Option<LintFix> {
    // `x > lower` cannot be represented directly in one `in start..end` expression
    if !range_check.lower_inclusive {
        return None;
    }

    let variable_text = ctx.get_span_text(ctx.tree.get_span(range_check.variable_id));
    let lower_text = ctx.get_span_text(ctx.tree.get_span(range_check.lower_bound_id));
    let upper_text = ctx.get_span_text(ctx.tree.get_span(range_check.upper_bound_id));
    let range_operator = if range_check.upper_inclusive {
        "..="
    } else {
        ".."
    };

    let replacement = format!("{variable_text} in {lower_text}{range_operator}{upper_text}");
    let edits = ctx
        .edit_builder()
        .replace(ctx.tree.get_span(expression_id), replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Replace with range contains expression").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_range_check_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_range_check_detected.ds",
            r#"
function foo(x: int32) {
    if x >= 0 && x < 10 {
        doX()
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-range-contains");
    }

    #[test]
    fn test_fix_rewrites_inclusive_exclusive_range_check() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_fix_rewrites_inclusive_exclusive_range_check.ds",
            r#"
function foo(x: int32) {
    if x >= 0 && x < 10 {
        doX()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-range-contains")
            .assert_has_fix("prefer-range-contains")
            .assert_unsafe_fixed(
                r#"
function foo(x: int32) {
    if (x in 0..10) {
        doX()
    }
}
"#,
            );
    }

    #[test]
    fn test_range_check_greater_less_or_equal() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_range_check_greater_less_or_equal.ds",
            r#"
function foo(x: int32) {
    if x > 0 && x <= 10 {
        doX()
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-range-contains");
    }

    #[test]
    fn test_mutation_fix_rewrites_inclusive_inclusive_range_check() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_mutation_fix_rewrites_inclusive_inclusive_range_check.ds",
            r#"
function foo(x: int32) {
    if x >= 0 && x <= 10 {
        doX()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-range-contains")
            .assert_has_fix("prefer-range-contains")
            .assert_unsafe_fixed(
                r#"
function foo(x: int32) {
    if (x in 0..=10) {
        doX()
    }
}
"#,
            );
    }

    #[test]
    fn test_no_fix_for_exclusive_lower_bound() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_no_fix_for_exclusive_lower_bound.ds",
            r#"
function foo(x: int32) {
    if x > 0 && x <= 10 {
        doX()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-range-contains")
            .assert_has_no_fix("prefer-range-contains");
    }

    #[test]
    fn test_range_in_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_range_in_allowed.ds",
            r#"
function foo(x: int32) {
    if x in 0..10 {
        doX()
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-range-contains");
    }

    #[test]
    fn test_different_variables_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_different_variables_allowed.ds",
            r#"
function foo(x: int32, y: int32) {
    if x >= 0 && y < 10 {
        doX()
    }
}
"#,
        );
        // different variables, not a range check
        test.result(result).assert_no_lint("prefer-range-contains");
    }

    #[test]
    fn test_single_comparison_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_single_comparison_allowed.ds",
            r#"
function foo(x: int32) {
    if x >= 0 {
        doX()
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-range-contains");
    }

    #[test]
    fn test_two_lower_bounds_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_two_lower_bounds_allowed.ds",
            r#"
function foo(x: int32) {
    if x >= 0 && x >= 5 {
        doX()
    }
}
"#,
        );
        // both are lower bounds, not a range check
        test.result(result).assert_no_lint("prefer-range-contains");
    }

    #[test]
    fn test_two_upper_bounds_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_two_upper_bounds_allowed.ds",
            r#"
function foo(x: int32) {
    if x < 10 && x < 20 {
        doX()
    }
}
"#,
        );
        // both are upper bounds, not a range check
        test.result(result).assert_no_lint("prefer-range-contains");
    }

    #[test]
    fn test_or_expression_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeContains);
        let result = test.lint_ast(
            "prefer_range_contains/test_or_expression_allowed.ds",
            r#"
function foo(x: int32) {
    if x >= 0 || x < 10 {
        doX()
    }
}
"#,
        );
        // using || not &&
        test.result(result).assert_no_lint("prefer-range-contains");
    }
}
