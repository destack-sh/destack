use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::{
    argument_value_expression_id, expression_path_segments, type_expression_path_segments,
};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow placeholder implementations.
    ///
    /// Throwing "not implemented" or similar messages indicates incomplete code.
    /// Implement the functionality or use a proper stub pattern.
    #[lint(
        id = "no-placeholder-implementation",
        code = "LR021",
        category = Restriction,
        level = Dir,
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let dir::Expression::Throw { value } = expression else {
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
                        ctx.dir.get_span(node_id),
                    )
                    .label("implement the functionality"),
                );
            }
        }
    }
}

/// Return one lowercase placeholder message from a throw payload expression.
fn extract_placeholder_message(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<String> {
    let expression = ctx.dir.get(expression_id);

    // match direct string literals
    if let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(value)) = expression {
        return Some(ctx.strings.get(*value).to_ascii_lowercase());
    }

    // match static template literals
    if let dir::Expression::TemplateExpression { value } = expression
        && let dir::TemplateLiteral::String { string } = value
    {
        return Some(ctx.strings.get(*string).to_ascii_lowercase());
    }

    // match Error constructor or call payloads
    let (is_error_constructor, arguments) = match expression {
        dir::Expression::New { ty, arguments } => (
            type_expression_is_error_constructor(ctx, *ty),
            arguments.as_slice(),
        ),
        dir::Expression::Call {
            left, arguments, ..
        } => (
            expression_is_error_constructor(ctx, *left),
            arguments.as_slice(),
        ),
        _ => return None,
    };
    if !is_error_constructor {
        return None;
    }
    let first_argument_id = arguments.first().copied()?;
    let first_value_expression_id =
        argument_value_expression_id(ctx.dir.tree(), first_argument_id)?;
    let first_value_expression = ctx.dir.get(first_value_expression_id);

    // enforce this lint guard
    if let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(value)) =
        first_value_expression
    {
        return Some(ctx.strings.get(*value).to_ascii_lowercase());
    }
    if let dir::Expression::TemplateExpression { value } = first_value_expression
        && let dir::TemplateLiteral::String { string } = value
    {
        return Some(ctx.strings.get(*string).to_ascii_lowercase());
    }

    None
}

/// Return true when one type expression names one error constructor or helper.
fn type_expression_is_error_constructor(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let Some(segments) = type_expression_path_segments(ctx.dir.tree(), type_expression_id) else {
        return false;
    };

    path_segments_name_error(ctx, segments.as_slice())
}

/// Return true when one expression names one error constructor or helper.
fn expression_is_error_constructor(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(segments) = expression_path_segments(ctx.dir.tree(), expression_id) else {
        return false;
    };

    path_segments_name_error(ctx, segments.as_slice())
}

/// Return true when one path ends in an Error constructor name.
fn path_segments_name_error(ctx: &LintModuleContext<'_>, segments: &[dir::StringId]) -> bool {
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
        let result = test.lint(
            "no_placeholder_implementation/test_detects_throw_not_implemented.ts",
            r#"throw "not implemented";"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_todo() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint(
            "no_placeholder_implementation/test_detects_throw_todo.ts",
            r#"throw "TODO: implement this";"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_new_error() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint(
            "no_placeholder_implementation/test_detects_throw_new_error.ts",
            r#"throw new Error("not implemented");"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_allows_real_error() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint(
            "no_placeholder_implementation/test_allows_real_error.ts",
            r#"throw new Error("Invalid input");"#,
        );
        test.result(result)
            .assert_no_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_allows_throw_variable() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint(
            "no_placeholder_implementation/test_allows_throw_variable.ts",
            "throw error;",
        );
        test.result(result)
            .assert_no_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_type_error_placeholder() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint(
            "no_placeholder_implementation/test_detects_throw_type_error_placeholder.ts",
            r#"throw new TypeError("TODO: add implementation");"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_error_call_placeholder() {
        let test = TestProgram::for_rule_without_prelude(NoPlaceholderImplementation);
        let result = test.lint(
            "no_placeholder_implementation/test_detects_throw_error_call_placeholder.ts",
            r#"throw Error("not yet implemented");"#,
        );
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }
}
