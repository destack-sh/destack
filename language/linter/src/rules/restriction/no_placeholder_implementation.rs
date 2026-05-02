use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{argument_value_expression_id, expression_path_segments};
use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow placeholder implementations.
    ///
    /// Throwing "not implemented" or similar messages indicates incomplete code.
    /// Implement the functionality or use a proper stub pattern.
    #[lint(
        id = "no-placeholder-implementation",
        code = "LR021",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoPlaceholderImplementation,
    "Disallow placeholder implementations"
}

// common placeholder messages
const PLACEHOLDER_PATTERNS: &[&str] = &[
    "not implemented",
    "not yet implemented",
    "todo",
    "fixme",
    "unimplemented",
    "stub",
    "placeholder",
];

impl LintRule for NoPlaceholderImplementation {
    fn meta(&self) -> &'static LintMeta {
        NoPlaceholderImplementation::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::Throw { value } = expression else {
                continue;
            };

            // check if the thrown value has one placeholder message
            let message = extract_placeholder_message(ctx, *value);
            let Some(message) = message else {
                continue;
            };

            // resolve is placeholder
            let is_placeholder = PLACEHOLDER_PATTERNS
                .iter()
                .any(|pattern| message.contains(pattern));
            if is_placeholder {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintReport::new(
                        NO_PLACEHOLDER_IMPLEMENTATION.id,
                        NO_PLACEHOLDER_IMPLEMENTATION.code,
                        NO_PLACEHOLDER_IMPLEMENTATION.category,
                        severity,
                        "placeholder implementation",
                        ctx.tree.get_span(node_id),
                    )
                    .label("implement the functionality"),
                );
            }
        }
    }
}

/// Return one lowercase placeholder message from a throw payload expression.
fn extract_placeholder_message(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<String> {
    let expression = ctx.tree.get(expression_id);

    // match direct string literals
    if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(value)) = expression {
        return Some(ctx.strings.get(*value).to_ascii_lowercase());
    }

    // match static template literals
    if let ast::Expression::TemplateExpression { value } = expression
        && let ast::TemplateLiteral::String { string } = value
    {
        return Some(ctx.strings.get(*string).to_ascii_lowercase());
    }

    // match Error constructor or call payloads
    let (callee_id, arguments) = match expression {
        ast::Expression::New {
            left, arguments, ..
        }
        | ast::Expression::Call {
            left, arguments, ..
        } => (left, arguments),
        _ => return None,
    };
    if !expression_is_error_constructor(ctx, *callee_id) {
        return None;
    }
    let first_argument_id = arguments.first().copied()?;
    let first_value_expression_id = argument_value_expression_id(ctx.tree, first_argument_id)?;
    let first_value_expression = ctx.tree.get(first_value_expression_id);

    // enforce this lint guard
    if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(value)) =
        first_value_expression
    {
        return Some(ctx.strings.get(*value).to_ascii_lowercase());
    }
    if let ast::Expression::TemplateExpression { value } = first_value_expression
        && let ast::TemplateLiteral::String { string } = value
    {
        return Some(ctx.strings.get(*string).to_ascii_lowercase());
    }

    None
}

/// Return true when one expression names one error constructor or helper.
fn expression_is_error_constructor(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let Some(segments) = expression_path_segments(ctx.tree, expression_id) else {
        return false;
    };
    let Some(last_segment) = segments.last().copied() else {
        return false;
    };
    let last_name = ctx.strings.get(last_segment);
    if last_name == "Error" {
        return true;
    }

    last_name.ends_with("Error")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_throw_not_implemented() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint_ast(
            "no_placeholder_implementation/test_detects_throw_not_implemented.ts",
            r#"throw "not implemented";"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_todo() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint_ast(
            "no_placeholder_implementation/test_detects_throw_todo.ts",
            r#"throw "TODO: implement this";"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_new_error() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint_ast(
            "no_placeholder_implementation/test_detects_throw_new_error.ts",
            r#"throw new Error("not implemented");"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_allows_real_error() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint_ast(
            "no_placeholder_implementation/test_allows_real_error.ts",
            r#"throw new Error("Invalid input");"#,
        );
        test.result(result)
            .assert_no_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_allows_throw_variable() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint_ast(
            "no_placeholder_implementation/test_allows_throw_variable.ts",
            "throw error;",
        );
        test.result(result)
            .assert_no_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_type_error_placeholder() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint_ast(
            "no_placeholder_implementation/test_detects_throw_type_error_placeholder.ts",
            r#"throw new TypeError("TODO: add implementation");"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_error_call_placeholder() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint_ast(
            "no_placeholder_implementation/test_detects_throw_error_call_placeholder.ts",
            r#"throw Error("not yet implemented");"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }
}
