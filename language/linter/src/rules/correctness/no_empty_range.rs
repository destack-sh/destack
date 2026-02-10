use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty ranges where start > end.
    ///
    /// A range like `10..5` is empty because the start is greater than
    /// the end. This is almost always a mistake.
    #[lint(
        id = "no-empty-range",
        code = "LC013",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoEmptyRange,
    "Disallow empty ranges"
}

impl LintRule for NoEmptyRange {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEmptyRange::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } = expression
            else {
                continue;
            };

            // try to get numeric values for start and end
            let start_value = get_numeric_value(ctx, *start);
            let end_value = get_numeric_value(ctx, *end);
            let (Some(start_val), Some(end_val)) = (start_value, end_value) else {
                continue;
            };

            // check if the range is empty
            let is_empty = if *is_inclusive {
                // for inclusive ranges (start..=end), empty if start > end
                start_val > end_val
            } else {
                // for exclusive ranges (start..end), empty if start >= end
                start_val >= end_val
            };

            if is_empty {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let range_type = if *is_inclusive { "..=" } else { ".." };
                let span = ctx.tree.get_span(node_id);
                let mut diagnostic = LintDiagnostic::new(
                    NO_EMPTY_RANGE.id,
                    NO_EMPTY_RANGE.code,
                    NO_EMPTY_RANGE.category,
                    severity,
                    format!("empty range: {start_val}{range_type}{end_val}"),
                    ctx.module.file_id,
                    span,
                )
                .with_label("this range contains no elements");

                // reverse bounds for clearly inverted ranges
                if ctx.compute_fixes
                    && let Some(fix) =
                        build_empty_range_fix(ctx, *start, *end, *is_inclusive, start_val, end_val)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build an unsafe fix for one inverted range.
fn build_empty_range_fix(
    ctx: &LintModuleAstContext<'_>,
    start_id: ast::LocalNodeId<Expression>,
    end_id: ast::LocalNodeId<Expression>,
    is_inclusive: bool,
    start_value: f64,
    end_value: f64,
) -> Option<LintFix> {
    // equal exclusive ranges are ambiguous: do not guess intent
    if !is_inclusive && start_value == end_value {
        return None;
    }

    let start_text = ctx.get_span_text(ctx.tree.get_span(start_id));
    let end_text = ctx.get_span_text(ctx.tree.get_span(end_id));
    let operator = if is_inclusive { "..=" } else { ".." };
    let replacement = format!("{end_text}{operator}{start_text}");
    let edit = ctx
        .edit_builder()
        .replace(
            ctx.tree.get_span(start_id).merge(ctx.tree.get_span(end_id)),
            replacement,
        )
        .into_edits();

    Some(LintFix::r#unsafe("Swap range bounds").with_edits(edit))
}

/// try to extract a numeric value from an expression
fn get_numeric_value(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<Expression>,
) -> Option<f64> {
    let expression = ctx.tree.get(expr_id);
    match expression {
        Expression::ScalarLiteral(ScalarLiteral::Integer(n)) => Some(*n as f64),
        Expression::ScalarLiteral(ScalarLiteral::Float(n)) => Some(*n),
        Expression::Unary { operator, right } => {
            // handle unary minus
            if *operator == ast::UnaryOperator::Negate {
                get_numeric_value(ctx, *right).map(|n| -n)
            } else {
                None
            }
        }
        Expression::Parenthesized { expression } => get_numeric_value(ctx, *expression),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_exclusive_range() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_detects_empty_exclusive_range.ds",
            r#"
let range = 10..5
"#,
        );
        test.result(result).assert_lint("no-empty-range");
    }

    #[test]
    fn test_fix_swaps_exclusive_range_bounds() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_fix_swaps_exclusive_range_bounds.ds",
            r#"
let range = 10..5
"#,
        );
        test.result(result)
            .assert_lint("no-empty-range")
            .assert_has_fix("no-empty-range")
            .assert_unsafe_fixed(
                r#"
let range = 5..10;
"#,
            );
    }

    #[test]
    fn test_mutation_fix_swaps_inclusive_range_bounds() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_mutation_fix_swaps_inclusive_range_bounds.ds",
            r#"
let range = 10..=5
"#,
        );
        test.result(result)
            .assert_lint("no-empty-range")
            .assert_has_fix("no-empty-range")
            .assert_unsafe_fixed(
                r#"
let range = 5..=10;
"#,
            );
    }

    #[test]
    fn test_detects_empty_equal_exclusive_range() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_detects_empty_equal_exclusive_range.ds",
            r#"
let range = 5..5
"#,
        );
        test.result(result)
            .assert_lint("no-empty-range")
            .assert_has_no_fix("no-empty-range");
    }

    #[test]
    fn test_detects_empty_inclusive_range() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_detects_empty_inclusive_range.ds",
            r#"
let range = 10..=5
"#,
        );
        test.result(result).assert_lint("no-empty-range");
    }

    #[test]
    fn test_allows_valid_exclusive_range() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_allows_valid_exclusive_range.ds",
            r#"
let range = 1..10
"#,
        );
        test.result(result).assert_no_lint("no-empty-range");
    }

    #[test]
    fn test_allows_valid_inclusive_range() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_allows_valid_inclusive_range.ds",
            r#"
let range = 1..=10
"#,
        );
        test.result(result).assert_no_lint("no-empty-range");
    }

    #[test]
    fn test_allows_single_element_inclusive_range() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_allows_single_element_inclusive_range.ds",
            r#"
let range = 5..=5
"#,
        );
        test.result(result).assert_no_lint("no-empty-range");
    }

    #[test]
    fn test_detects_negative_empty_range() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyRange);
        let result = test.lint_ast(
            "no_empty_range/test_detects_negative_empty_range.ds",
            r#"
let range = -5..-10
"#,
        );
        test.result(result).assert_lint("no-empty-range");
    }
}
