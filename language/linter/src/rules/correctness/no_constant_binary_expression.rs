use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_constant_to_bool, expression_has_side_effects, expression_is_equal,
    expression_path_segments, expression_unwrap_parenthesized_source_form,
};
use crate::{LintAstContext, LintDiagnostic, LintMeta, LintRule, declare_lint};

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
        level = Ast,
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

    /// Check module AST nodes for binary expressions with constant outcomes.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // walk binary expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // check for constant outcomes
            if let Some(message) = check_constant_result(ctx, *left, *operator, *right) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.tree.get_span(node_id);
                ctx.report(
                    LintDiagnostic::new(
                        NO_CONSTANT_BINARY_EXPRESSION.id,
                        NO_CONSTANT_BINARY_EXPRESSION.code,
                        NO_CONSTANT_BINARY_EXPRESSION.category,
                        severity,
                        message,
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("this expression always produces the same result"),
                );
            }
        }
    }
}

/// Check if a binary expression produces a constant result.
fn check_constant_result(
    ctx: &LintAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    operator: ast::BinaryOperator,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> Option<&'static str> {
    // normalize expression shape
    let left_id = expression_unwrap_parenthesized_source_form(ctx.tree, left_id);
    let right_id = expression_unwrap_parenthesized_source_form(ctx.tree, right_id);

    // resolve expression references
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    // detect no op logical operations on the same side effect free value
    if matches!(
        operator,
        ast::BinaryOperator::Or | ast::BinaryOperator::And | ast::BinaryOperator::Coalesce
    ) && expression_is_equal(ctx, left_id, right_id)
        && !expression_has_side_effects(ctx, left_id)
        && !expression_has_side_effects(ctx, right_id)
    {
        return Some("logical operation on identical operands");
    }

    // detect constant short circuit behavior from the left operand truthiness
    if let Some(left_boolean) = expression_constant_truthiness(ctx, left_id) {
        if operator == ast::BinaryOperator::Or && left_boolean {
            return Some("logical OR short-circuits to a constant result");
        }

        // enforce this lint guard
        if operator == ast::BinaryOperator::And && !left_boolean {
            return Some("logical AND short-circuits to a constant result");
        }
    }

    // detect nullish coalescing with statically known left nullishness
    if operator == ast::BinaryOperator::Coalesce {
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
        ast::BinaryOperator::EqualStrict | ast::BinaryOperator::NotEqualStrict
    ) && matches!(left, ast::Expression::New { .. })
        && matches!(right, ast::Expression::New { .. })
    {
        return Some("comparing two new objects always produces the same result");
    }

    // check for `{} === {}` or `[] === []` (always false)
    if matches!(
        operator,
        ast::BinaryOperator::Equal
            | ast::BinaryOperator::NotEqual
            | ast::BinaryOperator::EqualStrict
            | ast::BinaryOperator::NotEqualStrict
    ) {
        let left_is_object = matches!(
            left,
            ast::Expression::ObjectExpression { .. } | ast::Expression::ArrayExpression { .. }
        );
        let right_is_object = matches!(
            right,
            ast::Expression::ObjectExpression { .. } | ast::Expression::ArrayExpression { .. }
        );
        if left_is_object && right_is_object {
            return Some("comparing two object literals always produces the same result");
        }
    }

    // check for string + undefined or string + null
    if operator == ast::BinaryOperator::Add {
        let left_is_string = matches!(
            left,
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_))
                | ast::Expression::TemplateExpression { .. }
        );
        let right_is_nullish = is_nullish(ctx.tree, right);
        if left_is_string && right_is_nullish {
            return Some("string concatenation with null/undefined");
        }
        let right_is_string = matches!(
            right,
            ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_))
                | ast::Expression::TemplateExpression { .. }
        );
        let left_is_nullish = is_nullish(ctx.tree, left);
        if right_is_string && left_is_nullish {
            return Some("string concatenation with null/undefined");
        }
    }

    None
}

/// Return one constant truthiness value when known by construction.
fn expression_constant_truthiness(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<bool> {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);

    // resolve direct constant values
    if let Some(boolean_value) = expression_constant_to_bool(ctx, expression) {
        return Some(boolean_value);
    }

    // resolve truthiness from expression shapes that are always truthy
    match expression {
        ast::Expression::ArrayExpression { .. }
        | ast::Expression::ObjectExpression { .. }
        | ast::Expression::New { .. }
        | ast::Expression::Declaration(_)
        | ast::Expression::ImportMeta
        | ast::Expression::This
        | ast::Expression::Super => Some(true),
        _ => None,
    }
}

/// Check if an expression is nullish (null or undefined).
fn is_nullish(tree: &ast::Tree, expression: &ast::Expression) -> bool {
    let ast::Expression::Type { value } = expression else {
        return false;
    };

    type_expression_is_nullish(tree, *value)
}

/// Return true when one type-expression node is `null` or `undefined`.
fn type_expression_is_nullish(
    tree: &ast::Tree,
    type_expression_id: ast::LocalNodeId<ast::TypeExpression>,
) -> bool {
    matches!(
        tree.get(type_expression_id),
        ast::TypeExpression::Literal {
            value: ast::TypeLiteral::Null | ast::TypeLiteral::Undefined,
        }
    )
}

/// Return true when nullishness of one expression is statically fixed.
fn expression_has_constant_nullishness(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    require_non_nullish: bool,
) -> bool {
    // normalize expression shape
    let expression_id = expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);

    // keep non nullish mode strict for nullish literals
    if require_non_nullish && is_nullish(ctx.tree, expression) {
        return false;
    }

    // branch by expression kind
    match expression {
        // literals have fixed nullishness
        ast::Expression::ScalarLiteral(_)
        | ast::Expression::Type { .. }
        | ast::Expression::ArrayExpression { .. }
        | ast::Expression::ObjectExpression { .. }
        | ast::Expression::TemplateExpression { .. }
        | ast::Expression::New { .. }
        | ast::Expression::Declaration(_) => true,

        // global casts have fixed nullishness
        ast::Expression::Call { left, .. } => expression_is_global_cast_call(ctx, *left),

        // unary operators produce non nullish scalar results
        ast::Expression::Unary { operator, .. } => *operator != ast::UnaryOperator::Void,

        // most binary expressions produce non nullish scalar results
        ast::Expression::Binary {
            operator: ast::BinaryOperator::Coalesce,
            right,
            ..
        } => expression_has_constant_nullishness(ctx, *right, true),
        ast::Expression::Binary { .. } => true,

        _ => false,
    }
}

/// Return true when one call target is a global cast builtin.
fn expression_is_global_cast_call(
    ctx: &LintAstContext<'_>,
    callee_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // resolve path segments
    let Some(path_segments) = expression_path_segments(ctx.tree, callee_id) else {
        return false;
    };
    let [name] = path_segments.as_slice() else {
        return false;
    };

    // match the known global cast names
    let name = ctx.strings.get(*name);
    name.as_ref() == "Boolean" || name.as_ref() == "String" || name.as_ref() == "Number"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_logical_or_same_literal() {
        let test = TestProgram::for_rule_without_prelude(NoConstantBinaryExpression);
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
