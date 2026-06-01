use destack_dir::{
    self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, TemplateLiteral, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{expression_type_or_call_return_type_map, is_string_type};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary template literal interpolation.
    ///
    /// A template literal like `${value}` is redundant when `value` is already
    /// string typed.
    #[lint(
        id = "no-unnecessary-template-expression",
        code = "LY068",
        category = Style,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::String)],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUnnecessaryTemplateExpression,
    "Disallow unnecessary template literal interpolation"
}

impl LintRule for NoUnnecessaryTemplateExpression {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnnecessaryTemplateExpression::meta()
    }

    /// Check module DIR nodes for unnecessary template literal interpolation.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoUnnecessaryTemplateExpressionVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags unnecessary template expression interpolation.
struct NoUnnecessaryTemplateExpressionVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoUnnecessaryTemplateExpressionVisitor<'a, 'b> {
    /// Build a visitor for no-unnecessary-template-expression checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let string_symbol = ctx.language_item(LanguageItem::String);

        Self {
            ctx,
            meta,
            string_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Return true when this template literal is exactly `${value}`.
    fn template_single_interpolation_argument(
        &self,
        template: &TemplateLiteral,
    ) -> Option<dir::LocalNodeId<dir::Argument>> {
        let TemplateLiteral::InterpolatedString { strings, arguments } = template else {
            return None;
        };

        if arguments.len() != 1 {
            return None;
        }

        for string_id in strings {
            let text = self.ctx.strings.get(*string_id);
            if !text.is_empty() {
                return None;
            }
        }

        Some(arguments[0])
    }

    /// Return true when the expression is string typed.
    fn expression_is_string_typed(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // string literal expressions are always string typed
        if matches!(
            self.ctx.dir.get(expression_id),
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(_))
        ) {
            return true;
        }

        expression_type_or_call_return_type_map(self.ctx, expression_id, |ctx, type_id| {
            is_string_type(ctx, type_id, Some(self.string_symbol))
        })
        .unwrap_or(false)
    }

    /// Check one template expression node.
    fn check_template_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        template: &TemplateLiteral,
    ) {
        let Some(argument_id) = self.template_single_interpolation_argument(template) else {
            return;
        };
        let argument = self.ctx.dir.get(argument_id);
        let Some(argument_value) = argument.value() else {
            return;
        };
        if !self.expression_is_string_typed(argument_value) {
            return;
        }

        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            NO_UNNECESSARY_TEMPLATE_EXPRESSION.id,
            NO_UNNECESSARY_TEMPLATE_EXPRESSION.code,
            NO_UNNECESSARY_TEMPLATE_EXPRESSION.category,
            severity,
            "unnecessary template interpolation",
            span,
        )
        .label("this template expression can be replaced by the string value directly");

        // build a safe replacement from the interpolated expression
        if self.ctx.compute_fixes {
            let value_span = self.ctx.get_span(argument_value);
            let value_text = self.ctx.get_span_text(value_span).to_string();
            let edits = self
                .ctx
                .edit_builder()
                .replace(span, value_text)
                .into_edits();
            let fix = LintFix::safe("Remove unnecessary template interpolation").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        // report the redundant template interpolation
        self.ctx.report(diagnostic);
    }
}

impl NodeVisitor for NoUnnecessaryTemplateExpressionVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check template expressions
        if let dir::Expression::TemplateExpression { value } = expression {
            self.check_template_expression(id, value);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Report `${value}` when `value` is already string typed.
    #[test]
    fn test_flags_single_string_interpolation() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_flags_single_string_interpolation.ds",
            r#"
let name: string = "Ada";
let message = `${name}`;
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-template-expression");
    }

    /// Safely remove `${value}` when `value` is already string typed.
    #[test]
    fn test_fix_single_string_interpolation() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_fix_single_string_interpolation.ds",
            r#"
let name: string = "Ada";
let message = `${name}`;
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-template-expression")
            .assert_has_fix("no-unnecessary-template-expression")
            .assert_safe_fixed(
                r#"
let name: string = "Ada";
let message = name;
"#,
            );
    }

    /// Report `${value}` for imported string values.
    #[test]
    fn test_flags_cross_module_single_string_interpolation() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_template_expression/source.ds" => r#"
export const name: string = "Ada";
"#,
                "no_unnecessary_template_expression/consumer.ds" => r#"
import { name } from "./source.ds";
let message = `${name}`;
"#,
            },
            "no_unnecessary_template_expression/consumer.ds",
        );

        test.result(diagnostics)
            .assert_lint("no-unnecessary-template-expression");
    }

    /// Allow template literals with surrounding text.
    #[test]
    fn test_allows_template_with_surrounding_text() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_allows_template_with_surrounding_text.ds",
            r#"
let name: string = "Ada";
let message = `hello ${name}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-template-expression");
    }

    /// Allow interpolation of non-string values.
    #[test]
    fn test_allows_non_string_interpolation() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_allows_non_string_interpolation.ds",
            r#"
let count: int32 = 3;
let message = `${count}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-template-expression");
    }

    /// Allow templates with multiple interpolations.
    #[test]
    fn test_allows_multiple_interpolations() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_allows_multiple_interpolations.ds",
            r#"
let first: string = "hello";
let second: string = "world";
let message = `${first} ${second}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-template-expression");
    }

    /// Report `${value}` when the value comes from a typed local alias.
    #[test]
    fn test_flags_single_string_interpolation_for_local_alias() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_flags_single_string_interpolation_for_local_alias.ds",
            r#"
let name: string = "Ada";
let alias = name;
let message = `${alias}`;
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-template-expression");
    }

    /// Allow `${value}` when the imported value is not string typed.
    #[test]
    fn test_allows_cross_module_non_string_interpolation() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_unnecessary_template_expression/source_number.ds" => r#"
export const count: int32 = 3;
"#,
                "no_unnecessary_template_expression/consumer_number.ds" => r#"
import { count } from "./source_number.ds";
let message = `${count}`;
"#,
            },
            "no_unnecessary_template_expression/consumer_number.ds",
        );

        test.result(diagnostics)
            .assert_no_lint("no-unnecessary-template-expression");
    }

    /// Allow interpolation when the expression type is unknown.
    #[test]
    fn test_allows_unknown_type_interpolation() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_allows_unknown_type_interpolation.ds",
            r#"
let value: unknown = "Ada";
let message = `${value}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-template-expression");
    }

    /// Report template interpolation for string literals.
    #[test]
    fn test_flags_string_literal_interpolation() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_flags_string_literal_interpolation.ds",
            r#"
let message = `${"Ada"}`;
"#,
        );
        test.result(result)
            .assert_lint("no-unnecessary-template-expression");
    }

    /// Allow template interpolation for unions that can be non string.
    #[test]
    fn test_allows_union_interpolation_with_non_string_branch() {
        let test = TestProgram::for_rule_with_prelude(NoUnnecessaryTemplateExpression);
        let result = test.lint_dir(
            "no_unnecessary_template_expression/test_allows_union_interpolation_with_non_string_branch.ds",
            r#"
let value: string | int32 = 3;
let message = `${value}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-unnecessary-template-expression");
    }
}
