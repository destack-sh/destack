use destack_dir::{self as dir, WellKnownSymbol};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    expression_type_or_call_return_type_map, is_template_interpolation_type,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Restrict template interpolations to string or numeric values.
    ///
    /// Explicitly string or numeric interpolation keeps string formatting clear
    /// and avoids accidental implicit object stringification.
    #[lint(
        id = "restrict-template-expressions",
        code = "LY081",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::String)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub RestrictTemplateExpressions,
    "Restrict template expressions to strings and numbers"
}

impl LintRule for RestrictTemplateExpressions {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        RestrictTemplateExpressions::meta()
    }

    /// Check module DIR nodes for disallowed template interpolation types.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let string_symbol = ctx.well_known_symbol(WellKnownSymbol::String);

        for (_expression_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::TemplateExpression { value } = expression else {
                continue;
            };
            let dir::TemplateLiteral::InterpolatedString { arguments, .. } = value else {
                continue;
            };

            for argument_id in arguments {
                let argument = ctx.tree.get(*argument_id);
                let value_expression_id = argument.value();
                if expression_allows_template_interpolation(ctx, value_expression_id, string_symbol)
                {
                    continue;
                }

                let severity = ctx.get_effective_severity(meta, value_expression_id);
                if !severity.is_enabled() {
                    continue;
                }

                let span = ctx.get_span(value_expression_id);
                ctx.report(
                    LintReport::new(
                        RESTRICT_TEMPLATE_EXPRESSIONS.id,
                        RESTRICT_TEMPLATE_EXPRESSIONS.code,
                        RESTRICT_TEMPLATE_EXPRESSIONS.category,
                        severity,
                        "template interpolation should be string or number typed",
                        span,
                    )
                    .label("convert this value to string before interpolation"),
                );
            }
        }
    }
}

/// Return true when one template interpolation expression is allowed.
fn expression_allows_template_interpolation(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    string_symbol: dir::GlobalSymbolId,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    if matches!(
        expression,
        dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(_)
                | dir::ScalarLiteral::Bigint(_)
                | dir::ScalarLiteral::Float(_)
                | dir::ScalarLiteral::String(_),
        }
    ) {
        return true;
    }

    // use DIR expression types as the source of truth
    expression_type_or_call_return_type_map(
        ctx.artifacts.as_ref(),
        ctx.profile_id,
        ctx.module_id(),
        ctx.tree,
        ctx.types,
        expression_id,
        |types, type_id| is_template_interpolation_type(types, type_id, Some(string_symbol)),
    )
    .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Flag boolean interpolation in templates.
    #[test]
    fn test_flags_boolean_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_flags_boolean_interpolation.ds",
            r#"
let value: boolean = true;
let message = `value: ${value}`;
"#,
        );
        test.result(result)
            .assert_lint("restrict-template-expressions");
    }

    /// Flag object interpolation in templates.
    #[test]
    fn test_flags_object_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_flags_object_interpolation.ds",
            r#"
let user = { name: "Ada" };
let message = `value: ${user}`;
"#,
        );
        test.result(result)
            .assert_lint("restrict-template-expressions");
    }

    /// Allow string interpolation.
    #[test]
    fn test_allows_string_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_allows_string_interpolation.ds",
            r#"
let value: string = "Ada";
let message = `value: ${value}`;
"#,
        );
        test.result(result)
            .assert_no_lint("restrict-template-expressions");
    }

    /// Allow numeric interpolation.
    #[test]
    fn test_allows_numeric_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_allows_numeric_interpolation.ds",
            r#"
let value: int32 = 42;
let message = `value: ${value}`;
"#,
        );
        test.result(result)
            .assert_no_lint("restrict-template-expressions");
    }

    /// Allow bigint interpolation.
    #[test]
    fn test_allows_bigint_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_allows_bigint_interpolation.ds",
            r#"
let value: bigint = 42n;
let message = `value: ${value}`;
"#,
        );
        test.result(result)
            .assert_no_lint("restrict-template-expressions");
    }

    /// Allow string or number unions.
    #[test]
    fn test_allows_string_number_union_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_allows_string_number_union_interpolation.ds",
            r#"
let value: string | int32 = 42;
let message = `value: ${value}`;
"#,
        );
        test.result(result)
            .assert_no_lint("restrict-template-expressions");
    }

    /// Flag unions that include non-string and non-number values.
    #[test]
    fn test_flags_union_with_boolean_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_flags_union_with_boolean_interpolation.ds",
            r#"
let value: string | boolean = true;
let message = `value: ${value}`;
"#,
        );
        test.result(result)
            .assert_lint("restrict-template-expressions");
    }

    /// Allow imported string interpolation.
    #[test]
    fn test_allows_imported_string_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "restrict_template_expressions/source.ds" => r#"
export const name: string = "Ada";
"#,
                "restrict_template_expressions/consumer.ds" => r#"
import { name } from "./source.ds";

let message = `value: ${name}`;
"#,
            },
            "restrict_template_expressions/consumer.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("restrict-template-expressions");
    }

    /// Flag imported object interpolation.
    #[test]
    fn test_flags_imported_object_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "restrict_template_expressions/source.ds" => r#"
export const user = { name: "Ada" };
"#,
                "restrict_template_expressions/consumer.ds" => r#"
import { user } from "./source.ds";

let message = `value: ${user}`;
"#,
            },
            "restrict_template_expressions/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("restrict-template-expressions")
            .assert_lint_count("restrict-template-expressions", 1);
    }

    /// Flag interpolation when the type is unknown.
    #[test]
    fn test_flags_unknown_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_flags_unknown_interpolation.ds",
            r#"
let value: unknown = {};
let message = `value: ${value}`;
"#,
        );
        test.result(result)
            .assert_lint("restrict-template-expressions");
    }

    /// Allow interpolation for string-returning call expressions.
    #[test]
    fn test_allows_string_returning_call_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_allows_string_returning_call_interpolation.ds",
            r#"
function name(): string {
    return "Ada";
}

let message = `value: ${name()}`;
"#,
        );
        test.result(result)
            .assert_no_lint("restrict-template-expressions");
    }

    /// Flag interpolation for object-returning call expressions.
    #[test]
    fn test_flags_object_returning_call_interpolation() {
        let test = TestProgram::for_rule_with_prelude(RestrictTemplateExpressions);
        let result = test.lint_dir(
            "restrict_template_expressions/test_flags_object_returning_call_interpolation.ds",
            r#"
function user(): { name: string } {
    return { name: "Ada" };
}

let message = `value: ${user()}`;
"#,
        );
        test.result(result)
            .assert_lint("restrict-template-expressions");
    }
}
