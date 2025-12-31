use destack_ast::{self as ast, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        code = "LY060",
        category = Style,
        level = Ast,
        fixable = No,
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

            let left_expression = ctx.tree.get(*left);
            let right_expression = ctx.tree.get(*right);

            // check if this looks like a range check: x >= start && x < end
            if is_range_check_pattern(ctx, left_expression, right_expression) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        PREFER_RANGE_CONTAINS.id,
                        PREFER_RANGE_CONTAINS.code,
                        PREFER_RANGE_CONTAINS.category,
                        severity,
                        "use `in` operator with range instead of comparison chain",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("replace with `x in start..end`"),
                );
            }
        }
    }
}

/// Check if two expressions form a range check pattern.
/// Patterns like: x >= start && x < end, or x > start && x <= end
fn is_range_check_pattern(
    ctx: &LintModuleAstContext<'_>,
    left: &Expression,
    right: &Expression,
) -> bool {
    // get comparison info from both sides
    let left_cmp = get_comparison_info(ctx, left);
    let right_cmp = get_comparison_info(ctx, right);

    let (Some(left_info), Some(right_info)) = (left_cmp, right_cmp) else {
        return false;
    };

    // check if they're comparing the same variable
    if left_info.var_name != right_info.var_name {
        return false;
    }

    // check if one is a lower bound and one is an upper bound
    let has_lower_bound = matches!(
        left_info.kind,
        ComparisonKind::GreaterOrEqual | ComparisonKind::Greater
    ) || matches!(
        right_info.kind,
        ComparisonKind::GreaterOrEqual | ComparisonKind::Greater
    );

    let has_upper_bound = matches!(
        left_info.kind,
        ComparisonKind::LessOrEqual | ComparisonKind::Less
    ) || matches!(
        right_info.kind,
        ComparisonKind::LessOrEqual | ComparisonKind::Less
    );

    has_lower_bound && has_upper_bound
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ComparisonKind {
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

struct ComparisonInfo {
    var_name: String,
    kind: ComparisonKind,
}

/// Extract comparison info from a binary comparison expression.
fn get_comparison_info(
    ctx: &LintModuleAstContext<'_>,
    expression: &Expression,
) -> Option<ComparisonInfo> {
    let Expression::Binary {
        operator,
        left,
        right: _,
    } = expression
    else {
        return None;
    };

    let left_expression = ctx.tree.get(*left);

    // determine the comparison kind and which side has the variable
    let (var_name, kind) = match operator {
        BinaryOperator::LessThan => {
            // x < y: x is the variable
            let name = get_simple_path_name(ctx, left_expression)?;
            (name, ComparisonKind::Less)
        }
        BinaryOperator::LessThanOrEqual => {
            // x <= y: x is the variable
            let name = get_simple_path_name(ctx, left_expression)?;
            (name, ComparisonKind::LessOrEqual)
        }
        BinaryOperator::GreaterThan => {
            // x > y: x is the variable
            let name = get_simple_path_name(ctx, left_expression)?;
            (name, ComparisonKind::Greater)
        }
        BinaryOperator::GreaterThanOrEqual => {
            // x >= y: x is the variable
            let name = get_simple_path_name(ctx, left_expression)?;
            (name, ComparisonKind::GreaterOrEqual)
        }
        _ => return None,
    };

    Some(ComparisonInfo { var_name, kind })
}

/// Get the name of a simple path expression.
fn get_simple_path_name(ctx: &LintModuleAstContext<'_>, expression: &Expression) -> Option<String> {
    let Expression::Path { path, .. } = expression else {
        return None;
    };

    if path.segments.len() != 1 {
        return None;
    }

    Some(ctx.strings.get(path.segments[0]).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_range_check_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferRangeContains);
        let result = test.lint_ast(
            "test.ds",
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
    fn test_range_check_greater_less_or_equal() {
        let test = TestProgram::for_rule_without_builtins(PreferRangeContains);
        let result = test.lint_ast(
            "test.ds",
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
    fn test_range_in_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferRangeContains);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(PreferRangeContains);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(PreferRangeContains);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(PreferRangeContains);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(PreferRangeContains);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(PreferRangeContains);
        let result = test.lint_ast(
            "test.ds",
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
