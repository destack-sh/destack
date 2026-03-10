use destack_dir::{self as dir, BinaryOperator, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_is_numeric_literal, expression_unwrap_parenthesized};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow shorthand implicit coercions.
    ///
    /// Shorthand coercions like `!!value`, unary `+value`, and `"" + value`
    /// are terse but less explicit than dedicated conversion APIs.
    #[lint(
        id = "no-implicit-coercion",
        code = "LY073",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoImplicitCoercion,
    "Disallow shorthand implicit coercions"
}

impl LintRule for NoImplicitCoercion {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoImplicitCoercion::meta()
    }

    /// Check module DIR nodes for shorthand coercions.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        for (expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            // detect unary `+value` coercion
            if let dir::Expression::Unary {
                operator: UnaryOperator::Plus,
                right,
            } = expression
            {
                if expression_is_numeric_literal(ctx.tree, *right) {
                    continue;
                }

                report_implicit_coercion(
                    ctx,
                    meta,
                    expression_id,
                    "implicit numeric coercion",
                    "use `Number(value)` for explicit conversion",
                );
                continue;
            }

            // detect `!!value` coercion
            if let dir::Expression::Unary {
                operator: UnaryOperator::Not,
                right,
            } = expression
            {
                if is_double_negation(ctx.tree, *right) {
                    report_implicit_coercion(
                        ctx,
                        meta,
                        expression_id,
                        "implicit boolean coercion",
                        "use `Boolean(value)` for explicit conversion",
                    );
                }
                continue;
            }

            // detect `\"\" + value` and `value + \"\"` coercions
            if let dir::Expression::Binary {
                left,
                operator: BinaryOperator::Add,
                right,
            } = expression
            {
                let left_is_empty_string =
                    is_empty_string_expression(ctx.tree, &ctx.program.strings, *left);
                let right_is_empty_string =
                    is_empty_string_expression(ctx.tree, &ctx.program.strings, *right);

                if !left_is_empty_string && !right_is_empty_string {
                    continue;
                }

                if left_is_empty_string && is_string_literal_expression(ctx.tree, *right) {
                    continue;
                }
                if right_is_empty_string && is_string_literal_expression(ctx.tree, *left) {
                    continue;
                }

                report_implicit_coercion(
                    ctx,
                    meta,
                    expression_id,
                    "implicit string coercion",
                    "use `String(value)` for explicit conversion",
                );
            }
        }
    }
}

/// Report one no-implicit-coercion diagnostic.
fn report_implicit_coercion(
    ctx: &mut LintModuleDirContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
    message: &'static str,
    label: &'static str,
) {
    let severity = ctx.get_effective_severity(meta, expression_id);
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.get_span(expression_id);
    ctx.report(
        LintDiagnostic::new(
            NO_IMPLICIT_COERCION.id,
            NO_IMPLICIT_COERCION.code,
            NO_IMPLICIT_COERCION.category,
            severity,
            message,
            ctx.module.file_id,
            span,
        )
        .with_label(label),
    );
}

/// Return true when the expression is `!!value`.
fn is_double_negation(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    matches!(
        tree.get(expression_id),
        dir::Expression::Unary {
            operator: UnaryOperator::Not,
            ..
        }
    )
}

/// Return true when the expression is a string scalar literal.
fn is_string_literal_expression(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    matches!(
        tree.get(expression_id),
        dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(_)
        }
    )
}

/// Return true when the expression is an empty string literal.
fn is_empty_string_expression(
    tree: &dir::NodeTree,
    strings: &destack_core::StringPool,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    let dir::Expression::ScalarLiteral {
        value: dir::ScalarLiteral::String(string_id),
    } = expression
    else {
        return false;
    };

    strings.get(*string_id).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag boolean coercion via double negation.
    #[test]
    fn test_flags_double_negation() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_flags_double_negation.ds",
            r#"
let value: unknown = 1;
let is_set = !!value;
"#,
        );
        test.result(result).assert_lint("no-implicit-coercion");
    }

    /// Flag parenthesized double negation coercion.
    #[test]
    fn test_flags_parenthesized_double_negation() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_flags_parenthesized_double_negation.ds",
            r#"
let value: unknown = 1;
let is_set = !(!value);
"#,
        );
        test.result(result).assert_lint("no-implicit-coercion");
    }

    /// Flag numeric coercion via unary plus.
    #[test]
    fn test_flags_unary_plus_identifier() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_flags_unary_plus_identifier.ds",
            r#"
let value: string = "42";
let number_value = +value;
"#,
        );
        test.result(result).assert_lint("no-implicit-coercion");
    }

    /// Flag unary plus coercion with parenthesized expressions.
    #[test]
    fn test_flags_unary_plus_parenthesized_identifier() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_flags_unary_plus_parenthesized_identifier.ds",
            r#"
let value: string = "42";
let number_value = +(value);
"#,
        );
        test.result(result).assert_lint("no-implicit-coercion");
    }

    /// Allow unary plus for numeric literals.
    #[test]
    fn test_allows_unary_plus_numeric_literal() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_allows_unary_plus_numeric_literal.ds",
            r#"
let value = +1;
"#,
        );
        test.result(result).assert_no_lint("no-implicit-coercion");
    }

    /// Flag string coercion with empty prefix string.
    #[test]
    fn test_flags_empty_string_concat_left() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_flags_empty_string_concat_left.ds",
            r#"
let value: int32 = 7;
let text = "" + value;
"#,
        );
        test.result(result).assert_lint("no-implicit-coercion");
    }

    /// Flag string coercion with empty suffix string.
    #[test]
    fn test_flags_empty_string_concat_right() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_flags_empty_string_concat_right.ds",
            r#"
let value: int32 = 7;
let text = value + "";
"#,
        );
        test.result(result).assert_lint("no-implicit-coercion");
    }

    /// Allow regular string concatenation.
    #[test]
    fn test_allows_regular_string_concat() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_allows_regular_string_concat.ds",
            r#"
let left = "a";
let right = "b";
let text = left + right;
"#,
        );
        test.result(result).assert_no_lint("no-implicit-coercion");
    }

    /// Allow explicit empty-plus-string literal concatenation.
    #[test]
    fn test_allows_empty_string_concat_with_string_literal() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_allows_empty_string_concat_with_string_literal.ds",
            r#"
let text = "" + "hello";
"#,
        );
        test.result(result).assert_no_lint("no-implicit-coercion");
    }

    /// Allow regular numeric addition.
    #[test]
    fn test_allows_numeric_addition() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_allows_numeric_addition.ds",
            r#"
let sum = 1 + 2;
"#,
        );
        test.result(result).assert_no_lint("no-implicit-coercion");
    }

    /// Flag empty string concatenation with computed values.
    #[test]
    fn test_flags_empty_string_concat_with_call_expression() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_flags_empty_string_concat_with_call_expression.ds",
            r#"
function getValue(): int32 {
    return 7;
}

let text = "" + getValue();
"#,
        );
        test.result(result).assert_lint("no-implicit-coercion");
    }

    /// Allow explicit boolean conversion.
    #[test]
    fn test_allows_explicit_boolean_conversion() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_allows_explicit_boolean_conversion.ds",
            r#"
let value: unknown = 1;
let is_set = Boolean(value);
"#,
        );
        test.result(result).assert_no_lint("no-implicit-coercion");
    }

    /// Allow explicit numeric conversion.
    #[test]
    fn test_allows_explicit_numeric_conversion() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_allows_explicit_numeric_conversion.ds",
            r#"
let value: string = "42";
let number_value = Number(value);
"#,
        );
        test.result(result).assert_no_lint("no-implicit-coercion");
    }

    /// Allow explicit string conversion.
    #[test]
    fn test_allows_explicit_string_conversion() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitCoercion);
        let result = test.lint_dir(
            "no_implicit_coercion/test_allows_explicit_string_conversion.ds",
            r#"
let value: int32 = 7;
let text = String(value);
"#,
        );
        test.result(result).assert_no_lint("no-implicit-coercion");
    }
}
