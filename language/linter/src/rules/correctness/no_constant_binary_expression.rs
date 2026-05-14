use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_constant_to_bool, expression_has_side_effects, expression_is_equal,
    expression_path_segments, expression_unwrap_parenthesized_source_form,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow expressions where the operation doesn't affect the value.
    ///
    /// Binary expressions with certain operand combinations always produce the same
    /// result regardless of the input. This includes comparisons of a value to itself
    /// with certain operators, and operations that have no effect.
    #[lint(
        id = "no-constant-binary-expression",
        code = "LC008",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoConstantBinaryExpression,
    "Disallow expressions that always produce the same result"
}

impl LintRule for NoConstantBinaryExpression {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoConstantBinaryExpression::meta()
    }

    /// Check module source nodes for binary expressions with constant outcomes.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // walk binary expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.dir.get(node_id)
            else {
                continue;
            };

            // check for constant outcomes
            if let Some(message) = check_constant_result(ctx, *left, *operator, *right) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.dir.get_span(node_id);
                ctx.report(
                    LintReport::new(
                        NO_CONSTANT_BINARY_EXPRESSION.id,
                        NO_CONSTANT_BINARY_EXPRESSION.code,
                        NO_CONSTANT_BINARY_EXPRESSION.category,
                        severity,
                        message,
                        span,
                    )
                    .label("this expression always produces the same result"),
                );
            }
        }
    }
}

/// Check if a binary expression produces a constant result.
fn check_constant_result(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> Option<&'static str> {
    // normalize expression shape
    let left_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), left_id);
    let right_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), right_id);

    // resolve expression references
    let left = ctx.dir.get(left_id);
    let right = ctx.dir.get(right_id);

    // detect no op logical operations on the same side effect free value
    if matches!(
        operator,
        dir::BinaryOperator::Or | dir::BinaryOperator::And | dir::BinaryOperator::Coalesce
    ) && expression_is_equal(ctx, left_id, right_id)
        && !expression_has_side_effects(ctx, left_id)
        && !expression_has_side_effects(ctx, right_id)
    {
        return Some("logical operation on identical operands");
    }

    // detect constant short circuit behavior from the left operand truthiness
    if let Some(left_boolean) = expression_constant_truthiness(ctx, left_id) {
        if operator == dir::BinaryOperator::Or && left_boolean {
            return Some("logical OR short-circuits to a constant result");
        }

        // enforce this lint guard
        if operator == dir::BinaryOperator::And && !left_boolean {
            return Some("logical AND short-circuits to a constant result");
        }
    }

    // detect nullish coalescing with statically known left nullishness
    if operator == dir::BinaryOperator::Coalesce {
        if expression_has_constant_nullishness(ctx, left_id, false)
            && !expression_has_constant_nullishness(ctx, left_id, true)
        {
            return Some("nullish coalescing left operand is always nullish");
        }

        // enforce this lint guard
        if expression_has_constant_nullishness(ctx, left_id, true) {
            return Some("nullish coalescing left operand is never nullish");
        }
    }

    // check for `new X() === new X()` (always false for object comparisons)
    if matches!(
        operator,
        dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict
    ) && matches!(left, dir::Expression::New { .. })
        && matches!(right, dir::Expression::New { .. })
    {
        return Some("comparing two new objects always produces the same result");
    }

    // check for `{} === {}` or `[] === []` (always false)
    if matches!(
        operator,
        dir::BinaryOperator::Equal
            | dir::BinaryOperator::NotEqual
            | dir::BinaryOperator::EqualStrict
            | dir::BinaryOperator::NotEqualStrict
    ) {
        let left_is_object = matches!(
            left,
            dir::Expression::ObjectExpression { .. } | dir::Expression::ArrayExpression { .. }
        );
        let right_is_object = matches!(
            right,
            dir::Expression::ObjectExpression { .. } | dir::Expression::ArrayExpression { .. }
        );
        if left_is_object && right_is_object {
            return Some("comparing two object literals always produces the same result");
        }
    }

    // check for string + undefined or string + null
    if operator == dir::BinaryOperator::Add {
        let left_is_string = matches!(
            left,
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(_))
                | dir::Expression::TemplateExpression { .. }
        );
        let right_is_nullish = is_nullish(ctx.dir.tree(), right);
        if left_is_string && right_is_nullish {
            return Some("string concatenation with null/undefined");
        }
        let right_is_string = matches!(
            right,
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(_))
                | dir::Expression::TemplateExpression { .. }
        );
        let left_is_nullish = is_nullish(ctx.dir.tree(), left);
        if right_is_string && left_is_nullish {
            return Some("string concatenation with null/undefined");
        }
    }

    None
}

/// Return one constant truthiness value when known by construction.
fn expression_constant_truthiness(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<bool> {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);

    // resolve direct constant values
    if let Some(boolean_value) = expression_constant_to_bool(ctx, expression) {
        return Some(boolean_value);
    }

    // resolve truthiness from expression shapes that are always truthy
    match expression {
        dir::Expression::ArrayExpression { .. }
        | dir::Expression::ObjectExpression { .. }
        | dir::Expression::New { .. }
        | dir::Expression::Declaration(_)
        | dir::Expression::ImportMeta
        | dir::Expression::This
        | dir::Expression::Super => Some(true),
        _ => None,
    }
}

/// Check if an expression is nullish (null or undefined).
fn is_nullish(tree: &dir::Tree, expression: &dir::Expression) -> bool {
    let dir::Expression::Type { value } = expression else {
        return false;
    };

    type_expression_is_nullish(tree, *value)
}

/// Return true when one type-expression node is `null` or `undefined`.
fn type_expression_is_nullish(
    tree: &dir::Tree,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    matches!(
        tree.get(type_expression_id),
        dir::TypeExpression::Literal {
            value: dir::TypeLiteral::Null | dir::TypeLiteral::Undefined,
        }
    )
}

/// Return true when nullishness of one expression is statically fixed.
fn expression_has_constant_nullishness(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    require_non_nullish: bool,
) -> bool {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);

    // keep non nullish mode strict for nullish literals
    if require_non_nullish && is_nullish(ctx.dir.tree(), expression) {
        return false;
    }

    // branch by expression kind
    match expression {
        // literals have fixed nullishness
        dir::Expression::ScalarLiteral(_)
        | dir::Expression::Type { .. }
        | dir::Expression::ArrayExpression { .. }
        | dir::Expression::ObjectExpression { .. }
        | dir::Expression::TemplateExpression { .. }
        | dir::Expression::New { .. }
        | dir::Expression::Declaration(_) => true,

        // global casts have fixed nullishness
        dir::Expression::Call { left, .. } => expression_is_global_cast_call(ctx, *left),

        // unary operators produce non nullish scalar results
        dir::Expression::Unary { operator, .. } => *operator != dir::UnaryOperator::Void,

        // most binary expressions produce non nullish scalar results
        dir::Expression::Binary {
            operator: dir::BinaryOperator::Coalesce,
            right,
            ..
        } => expression_has_constant_nullishness(ctx, *right, true),
        dir::Expression::Binary { .. } => true,

        _ => false,
    }
}

/// Return true when one call target is a global cast builtin.
fn expression_is_global_cast_call(
    ctx: &LintModuleContext<'_>,
    callee_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // resolve path segments
    let Some(path_segments) = expression_path_segments(ctx.dir.tree(), callee_id) else {
        return false;
    };
    let [name] = path_segments.as_slice() else {
        return false;
    };

    // match the known global cast names
    let name = ctx.strings.get(*name);
    name == "Boolean" || name == "String" || name == "Number"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_logical_or_same_literal() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_logical_or_same_literal.ds",
            r#"
true || true;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_logical_and_same_literal() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_logical_and_same_literal.ds",
            r#"
false && false;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_new_object_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_new_object_comparison.ds",
            r#"
new Foo() === new Foo();
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_object_literal_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_object_literal_comparison.ds",
            r#"
const x = {} === {};
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_array_literal_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_array_literal_comparison.ds",
            r#"
[] === [];
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_string_plus_null() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_string_plus_null.ds",
            r#"
"hello" + null;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_string_plus_undefined() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_string_plus_undefined.ds",
            r#"
"hello" + undefined;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_different_literals_or() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_different_literals_or.ds",
            r#"
true || false;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_allows_variable_comparison() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_allows_variable_comparison.ds",
            r#"
let x = {};
let y = {};
x === y;
"#,
        );
        test.result(result)
            .assert_no_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_allows_string_concatenation() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_allows_string_concatenation.ds",
            r#"
"hello" + "world";
"#,
        );
        test.result(result)
            .assert_no_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_logical_or_short_circuit_true_left() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_logical_or_short_circuit_true_left.ds",
            r#"
true || compute();
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_logical_and_short_circuit_false_left() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_logical_and_short_circuit_false_left.ds",
            r#"
false && compute();
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_nullish_coalesce_non_nullish_left() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_nullish_coalesce_non_nullish_left.ds",
            r#"
"value" ?? fallback;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_nullish_coalesce_nullish_left() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_nullish_coalesce_nullish_left.ds",
            r#"
null ?? fallback;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_identical_variable_operands() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_identical_variable_operands.ds",
            r#"
let value = maybe();
value || value;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_allows_identical_call_operands_with_side_effects() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_allows_identical_call_operands_with_side_effects.ds",
            r#"
compute() || compute();
"#,
        );
        test.result(result)
            .assert_no_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_nullish_coalesce_unary_not_left() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_nullish_coalesce_unary_not_left.ds",
            r#"
!foo ?? fallback;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_nullish_coalesce_binary_left() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_nullish_coalesce_binary_left.ds",
            r#"
(a + b) ?? fallback;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_detects_logical_or_with_array_left() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_detects_logical_or_with_array_left.ds",
            r#"
[] || fallback;
"#,
        );
        test.result(result)
            .assert_lint("no-constant-binary-expression");
    }

    #[test]
    fn test_allows_logical_or_with_variable_left() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint(
            "no_constant_binary_expression/test_allows_logical_or_with_variable_left.ds",
            r#"
let value = maybe();
value || fallback;
"#,
        );
        test.result(result)
            .assert_no_lint("no-constant-binary-expression");
    }
}
