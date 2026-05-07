use destack_ast::{self as ast, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_path_segments, match_selector_expression_id};
use crate::{LintAstContext, LintFix, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Require `Number.isNaN()` instead of comparisons with `NaN`.
    ///
    /// `NaN` is unique in that it is not equal to itself. Comparisons like
    /// `x === NaN` or `x !== NaN` will never work as expected. Use
    /// `Number.isNaN(x)` instead.
    #[lint(
        id = "use-isnan",
        code = "LC043",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub UseIsnan,
    "Require Number.isNaN() instead of NaN comparisons"
}

impl LintRule for UseIsnan {
    fn meta(&self) -> &'static LintMeta {
        UseIsnan::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check binary comparison expressions
            if let Expression::Binary {
                left,
                operator,
                right,
            } = expression
            {
                check_binary_nan_comparison(ctx, meta, node_id, *left, *operator, *right);
                continue;
            }

            // check indexOf and lastIndexOf calls when enabled
            if ctx.options.correctness.use_isnan_enforce_for_index_of
                && let Expression::Call {
                    left, arguments, ..
                } = expression
            {
                check_index_of_nan_call(ctx, meta, node_id, *left, arguments);
                continue;
            }

            // check switch comparisons against NaN
            let Expression::Match { form, value, cases } = expression else {
                continue;
            };
            if *form != ast::MatchForm::Switch {
                continue;
            }
            if !ctx.options.correctness.use_isnan_enforce_for_switch_case {
                continue;
            }

            check_switch_nan_comparisons(ctx, meta, *value, cases);
        }
    }
}

/// Check one binary comparison for NaN usage.
fn check_binary_nan_comparison(
    ctx: &mut LintAstContext<'_>,
    meta: &LintMeta,
    node_id: ast::LocalNodeId<ast::Expression>,
    left: ast::LocalNodeId<ast::Expression>,
    operator: BinaryOperator,
    right: ast::LocalNodeId<ast::Expression>,
) {
    // only check comparison operators
    if !matches!(
        operator,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
    ) {
        return;
    }

    // check if either side is NaN
    let left_is_nan = is_nan_identifier(ctx, left);
    let right_is_nan = is_nan_identifier(ctx, right);
    if !left_is_nan && !right_is_nan {
        return;
    }

    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    let expression_span = ctx.tree.get_span(node_id);
    let mut diagnostic = LintReport::new(
        USE_ISNAN.id,
        USE_ISNAN.code,
        USE_ISNAN.category,
        severity,
        comparison_message(operator),
        expression_span,
    )
    .label("use Number.isNaN() instead");

    // construct fix for equality operators only
    if ctx.compute_fixes
        && let Some(fix) = make_isnan_fix(ctx, left, right, operator, left_is_nan, expression_span)
    {
        diagnostic = diagnostic.fix(fix);
    }

    ctx.report(diagnostic);
}

/// Check switch discriminants and case selectors for NaN comparisons.
fn check_switch_nan_comparisons(
    ctx: &mut LintAstContext<'_>,
    meta: &LintMeta,
    value: ast::LocalNodeId<ast::Expression>,
    cases: &[ast::LocalNodeId<ast::MatchCase>],
) {
    // switch discriminant `NaN` never equals any case selector
    if is_nan_identifier(ctx, value) {
        let severity = ctx.get_effective_severity(meta, value);
        if severity.is_enabled() {
            let span = ctx.tree.get_span(value);
            ctx.report(
                LintReport::new(
                    USE_ISNAN.id,
                    USE_ISNAN.code,
                    USE_ISNAN.category,
                    severity,
                    "switch discriminant is NaN",
                    span,
                )
                .label("switch cases are compared with `===`, so this never matches"),
            );
        }
    }

    // switch `case NaN` selector can never match
    for case_id in cases {
        let selector = ctx.tree.get(*case_id).selector();
        let Some(selector_expression_id) = match_selector_expression_id(ctx, selector) else {
            continue;
        };
        if !is_nan_identifier(ctx, selector_expression_id) {
            continue;
        }

        let severity = ctx.get_effective_severity(meta, selector_expression_id);
        if !severity.is_enabled() {
            continue;
        }

        let span = ctx.tree.get_span(selector_expression_id);
        ctx.report(
            LintReport::new(
                USE_ISNAN.id,
                USE_ISNAN.code,
                USE_ISNAN.category,
                severity,
                "switch case compares with NaN",
                span,
            )
            .label("switch cases are compared with `===`, so this case never matches"),
        );
    }
}

/// Check `indexOf` and `lastIndexOf` calls for `NaN`.
fn check_index_of_nan_call(
    ctx: &mut LintAstContext<'_>,
    meta: &LintMeta,
    node_id: ast::LocalNodeId<ast::Expression>,
    callee_id: ast::LocalNodeId<ast::Expression>,
    arguments: &[ast::LocalNodeId<ast::Argument>],
) {
    let Some(first_argument_id) = arguments.first() else {
        return;
    };
    let first_argument = ctx.tree.get(*first_argument_id);
    let argument_value_id = match first_argument {
        ast::Argument::Positional { value, .. } => *value,
        _ => return,
    };
    if !is_nan_identifier(ctx, argument_value_id) {
        return;
    }

    let Some(segments) = expression_path_segments(ctx.tree, callee_id) else {
        return;
    };
    if segments.len() < 2 {
        return;
    }

    let Some(method_name_id) = segments.last().copied() else {
        return;
    };
    let method_name = ctx.strings.get(method_name_id);
    if method_name.as_ref() != "indexOf" && method_name.as_ref() != "lastIndexOf" {
        return;
    }

    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.tree.get_span(node_id);
    ctx.report(
        LintReport::new(
            USE_ISNAN.id,
            USE_ISNAN.code,
            USE_ISNAN.category,
            severity,
            format!("{} cannot find NaN", method_name.as_ref()),
            span,
        )
        .label("use a Number.isNaN-aware search instead"),
    );
}

/// Check if an expression is the identifier `NaN` or `Number.NaN`
fn is_nan_identifier(ctx: &LintAstContext<'_>, expr_id: ast::LocalNodeId<ast::Expression>) -> bool {
    let expression = ctx.tree.get(expr_id);
    match expression {
        // unwrap parenthesized expressions before matching
        Expression::Parenthesized { expression } => is_nan_identifier(ctx, *expression),

        // handle sequence expressions by checking the last evaluated value
        Expression::SequenceExpression { expressions } => expressions
            .last()
            .is_some_and(|expression_id| is_nan_identifier(ctx, *expression_id)),

        // check reference forms: NaN or Number.NaN
        Expression::Identifier { .. }
        | Expression::QualifiedReference { .. }
        | Expression::Member { .. } => {
            let Some(segments) = expression_path_segments(ctx.tree, expr_id) else {
                return false;
            };
            if segments.as_slice().len() == 1 {
                return ctx.strings.get(segments[0]).as_ref() == "NaN";
            }
            if segments.as_slice().len() == 2 {
                let first = ctx.strings.get(segments[0]);
                let second = ctx.strings.get(segments[1]);
                return first.as_ref() == "Number" && second.as_ref() == "NaN";
            }

            false
        }
        _ => false,
    }
}

/// Return a diagnostic message matching the NaN comparison operator semantics.
fn comparison_message(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => {
            "comparison with NaN is always true"
        }
        BinaryOperator::Equal | BinaryOperator::EqualStrict => {
            "comparison with NaN is always false"
        }
        _ => "comparison with NaN is always false",
    }
}

/// Create a fix for NaN comparison (equality operators only).
fn make_isnan_fix(
    ctx: &LintAstContext<'_>,
    left: ast::LocalNodeId<ast::Expression>,
    right: ast::LocalNodeId<ast::Expression>,
    operator: BinaryOperator,
    left_is_nan: bool,
    expression_span: destack_source::Span,
) -> Option<LintFix> {
    // get the non-NaN operand
    let other_id = if left_is_nan { right } else { left };
    let other_span = ctx.tree.get_span(other_id);
    let other_text = ctx.get_span_text(other_span);

    // only strict equality operators are semantics preserving here
    let replacement = match operator {
        BinaryOperator::EqualStrict => format!("Number.isNaN({other_text})"),
        BinaryOperator::NotEqualStrict => format!("!Number.isNaN({other_text})"),

        // loose equality and relational operators: no fix
        _ => return None,
    };

    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();
    Some(LintFix::safe("Replace with Number.isNaN()").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nan_strict_equal() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_nan_strict_equal.ds",
            r#"
let x = 1.0
if (x === NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_nan_equal() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_nan_equal.ds",
            r#"
let x = 1.0
if (x == NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_does_not_fix_loose_equal() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_does_not_fix_loose_equal.ds",
            r#"
let x = 1.0
if (x == NaN) {}
"#,
        );
        test.result(result)
            .assert_lint("use-isnan")
            .assert_has_no_fix("use-isnan");
    }

    #[test]
    fn test_detects_nan_not_equal() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_nan_not_equal.ds",
            r#"
let x = 1.0
if (x !== NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_does_not_fix_loose_not_equal() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_does_not_fix_loose_not_equal.ds",
            r#"
let x = 1.0
if (x != NaN) {}
"#,
        );
        test.result(result)
            .assert_lint("use-isnan")
            .assert_has_no_fix("use-isnan");
    }

    #[test]
    fn test_detects_nan_less_than() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_nan_less_than.ds",
            r#"
let x = 1.0
if (x < NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_nan_on_left() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_nan_on_left.ds",
            r#"
let x = 1.0
if (NaN === x) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_number_nan() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_number_nan.ds",
            r#"
let x = 1.0
if (x === Number.NaN) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_nan_switch_discriminant() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_nan_switch_discriminant.ds",
            r#"
switch (NaN) {
    case 1: break
    default: break
}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_detects_nan_switch_case_selector() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_nan_switch_case_selector.ds",
            r#"
let x = 1.0
switch (x) {
    case NaN: break
    default: break
}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_allows_nan_switch_when_disabled() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan).with_options(|options| {
            options.correctness.use_isnan_enforce_for_switch_case = false;
        });
        let result = test.lint_ast(
            "use_isnan/test_allows_nan_switch_when_disabled.ds",
            r#"
switch (NaN) {
    case 1: break
    default: break
}
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_allows_normal_switch_cases() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_allows_normal_switch_cases.ds",
            r#"
let x = 1.0
switch (x) {
    case 1.0: break
    default: break
}
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_detects_sequence_expression_nan() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_detects_sequence_expression_nan.ts",
            r#"
let x = 1.0;
if (x === (0, NaN)) {}
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_allows_sequence_expression_without_nan_result() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_allows_sequence_expression_without_nan_result.ts",
            r#"
let x = 1.0;
if (x === (NaN, 0)) {}
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_allows_normal_comparison() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_allows_normal_comparison.ds",
            r#"
let x = 1.0
if (x === 0.0) {}
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_detects_index_of_nan_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan).with_options(|options| {
            options.correctness.use_isnan_enforce_for_index_of = true;
        });
        let result = test.lint_ast(
            "use_isnan/test_detects_index_of_nan_when_enabled.ds",
            r#"
let values = [1.0, 2.0];
let index = values.indexOf(NaN);
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_allows_index_of_nan_by_default() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_allows_index_of_nan_by_default.ds",
            r#"
let values = [1.0, 2.0];
let index = values.indexOf(NaN);
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_detects_last_index_of_nan_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan).with_options(|options| {
            options.correctness.use_isnan_enforce_for_index_of = true;
        });
        let result = test.lint_ast(
            "use_isnan/test_detects_last_index_of_nan_when_enabled.ds",
            r#"
let values = [1.0, 2.0];
let index = values.lastIndexOf(NaN);
"#,
        );
        test.result(result).assert_lint("use-isnan");
    }

    #[test]
    fn test_allows_bare_index_of_function_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan).with_options(|options| {
            options.correctness.use_isnan_enforce_for_index_of = true;
        });
        let result = test.lint_ast(
            "use_isnan/test_allows_bare_index_of_function_when_enabled.ds",
            r#"
function indexOf(value: number): number {
    return 0;
}

let index = indexOf(NaN);
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_allows_isnan_call() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_allows_isnan_call.ds",
            r#"
let x = 1.0
if (Number.isNaN(x)) {}
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_allows_nan_variable_name() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_allows_nan_variable_name.ds",
            r#"
let NaN = "not a number"
let x = NaN
"#,
        );
        test.result(result).assert_no_lint("use-isnan");
    }

    #[test]
    fn test_fix_strict_equal() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_fix_strict_equal.ds",
            r#"
let x = 1.0
if (x === NaN) {}
"#,
        );
        test.result(result)
            .assert_lint("use-isnan")
            .assert_safe_fixed(
                r#"
let x = 1.0;
if (Number.isNaN(x)) {
}
"#,
            );
    }

    #[test]
    fn test_fix_not_equal() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_fix_not_equal.ds",
            r#"
let x = 1.0
if (x !== NaN) {}
"#,
        );
        test.result(result)
            .assert_lint("use-isnan")
            .assert_safe_fixed(
                r#"
let x = 1.0;
if (!Number.isNaN(x)) {
}
"#,
            );
    }

    #[test]
    fn test_fix_nan_on_left() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_fix_nan_on_left.ds",
            r#"
let x = 1.0
if (NaN === x) {}
"#,
        );
        test.result(result)
            .assert_lint("use-isnan")
            .assert_safe_fixed(
                r#"
let x = 1.0;
if (Number.isNaN(x)) {
}
"#,
            );
    }

    #[test]
    fn test_fix_complex_expression() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_fix_complex_expression.ds",
            r#"
let y = 1.0
let x = (y + 1) === NaN
"#,
        );
        test.result(result)
            .assert_lint("use-isnan")
            .assert_safe_fixed(
                r#"
let y = 1.0;
let x = Number.isNaN((y + 1));
"#,
            );
    }

    #[test]
    fn test_no_fix_for_relational() {
        let test = TestProgram::for_rule_without_prelude(UseIsnan);
        let result = test.lint_ast(
            "use_isnan/test_no_fix_for_relational.ds",
            r#"
let x = 1.0
if (x < NaN) {}
"#,
        );
        test.result(result)
            .assert_lint("use-isnan")
            .assert_has_no_fix("use-isnan");
    }
}
