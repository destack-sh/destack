use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow nested template literals.
    ///
    /// Template literals nested inside other template literals are hard to read.
    /// Consider extracting the inner template to a variable or using
    /// string concatenation instead.
    #[lint(
        id = "no-nested-template-literal",
        code = "LY021",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNestedTemplateLiteral,
    "Disallow nested template literals"
}

impl LintRule for NoNestedTemplateLiteral {
    fn meta(&self) -> &'static LintMeta {
        NoNestedTemplateLiteral::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !is_template_expression(expression) {
                continue;
            }

            let Some(parent_template_id) = template_parent_expression(ctx, node_id) else {
                continue;
            };

            if !is_same_line_nested_template(ctx, node_id, parent_template_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintReport::new(
                    NO_NESTED_TEMPLATE_LITERAL.id,
                    NO_NESTED_TEMPLATE_LITERAL.code,
                    NO_NESTED_TEMPLATE_LITERAL.category,
                    severity,
                    "nested template literal",
                    ctx.tree.get_span(node_id),
                )
                .label("consider extracting to a variable"),
            );
        }
    }
}

/// Return true when one expression is a template literal expression.
fn is_template_expression(expression: &ast::Expression) -> bool {
    matches!(
        expression,
        ast::Expression::TemplateExpression { .. }
            | ast::Expression::TaggedTemplateExpression { .. }
    )
}

/// Resolve the nearest parent template expression for one template node.
fn template_parent_expression(
    ctx: &LintAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let mut current = expr_id.id;
    while let Some(parent_raw_id) = ctx.parents.get_by_id(current) {
        if ctx.tree.get_node_type(parent_raw_id) != ast::NodeType::Expression {
            current = parent_raw_id;
            continue;
        }

        let parent_id = ast::LocalNodeId::<ast::Expression>::new(parent_raw_id);
        let parent = ctx.tree.get(parent_id);
        if is_template_expression(parent) {
            return Some(parent_id);
        }

        current = parent_raw_id;
    }

    None
}

/// Return true when nested and parent templates share one boundary line.
fn is_same_line_nested_template(
    ctx: &LintAstContext<'_>,
    nested_template_id: ast::LocalNodeId<ast::Expression>,
    parent_template_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let nested_span = ctx.tree.get_span(nested_template_id);
    let parent_span = ctx.tree.get_span(parent_template_id);

    let nested_end = nested_span.end.saturating_sub(1);
    let parent_end = parent_span.end.saturating_sub(1);

    let Some((nested_start_line, _)) = ctx.file.get_position(nested_span.start) else {
        return false;
    };
    let Some((nested_end_line, _)) = ctx.file.get_position(nested_end) else {
        return false;
    };
    let Some((parent_start_line, _)) = ctx.file.get_position(parent_span.start) else {
        return false;
    };
    let Some((parent_end_line, _)) = ctx.file.get_position(parent_end) else {
        return false;
    };

    if nested_start_line == parent_start_line {
        return true;
    }

    if nested_end_line == parent_end_line {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nested_template() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "no_nested_template_literal/test_detects_nested_template.ds",
            r#"
let outer = `hello ${`world ${name}`}`;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-template-literal");
    }

    #[test]
    fn test_allows_simple_template() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "no_nested_template_literal/test_allows_simple_template.ds",
            r#"
let greeting = `hello ${name}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-nested-template-literal");
    }

    #[test]
    fn test_allows_sequential_templates() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "no_nested_template_literal/test_allows_sequential_templates.ds",
            r#"
let a = `hello ${name}`;
let b = `goodbye ${name}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-nested-template-literal");
    }

    #[test]
    fn test_allows_multiline_nested_template() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "no_nested_template_literal/test_allows_multiline_nested_template.ds",
            r#"
let message = `I have
${color ? `${count} ${color}` : count}
apples`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-nested-template-literal");
    }

    #[test]
    fn test_allows_template_in_function_call() {
        let test = TestProgram::for_rule_without_prelude(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "no_nested_template_literal/test_allows_template_in_function_call.ds",
            r#"
let x = `hello ${format(`${name}`)}`;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-template-literal");
    }
}
