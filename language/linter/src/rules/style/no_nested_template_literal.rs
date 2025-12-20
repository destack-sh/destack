use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNestedTemplateLiteral,
    "Disallow nested template literals"
}

impl LintRule for NoNestedTemplateLiteral {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNestedTemplateLiteral::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let is_template = matches!(
                expression,
                ast::Expression::TemplateExpression { .. }
                    | ast::Expression::TaggedTemplateExpression { .. }
            );
            if !is_template {
                continue;
            }

            // check if nested inside another template
            if is_nested_in_template(ctx, node_id) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_NESTED_TEMPLATE_LITERAL.id,
                        NO_NESTED_TEMPLATE_LITERAL.code,
                        NO_NESTED_TEMPLATE_LITERAL.category,
                        severity,
                        "nested template literal",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("consider extracting to a variable"),
                );
            }
        }
    }
}

/// Check if a template expression is nested inside another template.
fn is_nested_in_template(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let mut current = expr_id.id;
    while let Some(parent_raw_id) = ctx.parents.get_by_id(current) {
        let parent_type = ctx.tree.get_node_type(parent_raw_id);

        // check Expression nodes
        if parent_type == ast::NodeType::Expression {
            let parent_id = ast::LocalNodeId::<ast::Expression>::new(parent_raw_id);
            let parent = ctx.tree.get(parent_id);

            // check if parent is a template expression
            if matches!(
                parent,
                ast::Expression::TemplateExpression { .. }
                    | ast::Expression::TaggedTemplateExpression { .. }
            ) {
                return true;
            }

            // stop at function boundaries
            if let ast::Expression::Declaration(decl_id) = parent {
                let decl = ctx.tree.get(*decl_id);
                if matches!(decl, ast::Declaration::Function { .. }) {
                    return false;
                }
            }
        }

        current = parent_raw_id;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nested_template() {
        let test = TestProgram::for_rule_without_builtins(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
let outer = `hello ${`world ${name}`}`;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-template-literal");
    }

    #[test]
    fn test_allows_simple_template() {
        let test = TestProgram::for_rule_without_builtins(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
let greeting = `hello ${name}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-nested-template-literal");
    }

    #[test]
    fn test_allows_sequential_templates() {
        let test = TestProgram::for_rule_without_builtins(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
let a = `hello ${name}`;
let b = `goodbye ${name}`;
"#,
        );
        test.result(result)
            .assert_no_lint("no-nested-template-literal");
    }

    #[test]
    fn test_allows_template_in_function_call() {
        let test = TestProgram::for_rule_without_builtins(NoNestedTemplateLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = `hello ${format(`${name}`)}`;
"#,
        );
        test.result(result)
            .assert_lint("no-nested-template-literal");
    }
}
