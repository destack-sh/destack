use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow placeholder implementations.
    ///
    /// Throwing "not implemented" or similar messages indicates incomplete code.
    /// Implement the functionality or use a proper stub pattern.
    #[lint(
        id = "no-placeholder-implementation",
        code = "LR020",
        category = Restriction,
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        NoPlaceholderImplementation::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::Throw { value } = expression else {
                continue;
            };

            // check if the thrown value is a string literal with placeholder message
            let thrown = ctx.tree.get(*value);
            let message = match thrown {
                ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(s)) => {
                    Some(ctx.strings.get(*s).to_lowercase())
                }
                // also check `new Error("...")`
                ast::Expression::New {
                    dynamic_arguments, ..
                } => {
                    if dynamic_arguments.len() == 1 {
                        let arg = ctx.tree.get(dynamic_arguments[0]);
                        if let ast::Argument::Positional { value, .. } = arg {
                            let value_expr = ctx.tree.get(*value);
                            if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(s)) =
                                value_expr
                            {
                                Some(ctx.strings.get(*s).to_lowercase())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let Some(message) = message else {
                continue;
            };

            let is_placeholder = PLACEHOLDER_PATTERNS
                .iter()
                .any(|pattern| message.contains(pattern));
            if is_placeholder {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_PLACEHOLDER_IMPLEMENTATION.id,
                        NO_PLACEHOLDER_IMPLEMENTATION.code,
                        NO_PLACEHOLDER_IMPLEMENTATION.category,
                        severity,
                        "placeholder implementation",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("implement the functionality"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_throw_not_implemented() {
        let test = TestProgram::for_rule_without_builtins(NoPlaceholderImplementation);
        let result = test.lint_ast("test.ts", r#"throw "not implemented";"#);
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_todo() {
        let test = TestProgram::for_rule_without_builtins(NoPlaceholderImplementation);
        let result = test.lint_ast("test.ts", r#"throw "TODO: implement this";"#);
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_detects_throw_new_error() {
        let test = TestProgram::for_rule_without_builtins(NoPlaceholderImplementation);
        let result = test.lint_ast("test.ts", r#"throw new Error("not implemented");"#);
        test.result(result)
            .assert_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_allows_real_error() {
        let test = TestProgram::for_rule_without_builtins(NoPlaceholderImplementation);
        let result = test.lint_ast("test.ts", r#"throw new Error("Invalid input");"#);
        test.result(result)
            .assert_no_lint("no-placeholder-implementation");
    }

    #[test]
    fn test_allows_throw_variable() {
        let test = TestProgram::for_rule_without_builtins(NoPlaceholderImplementation);
        let result = test.lint_ast("test.ts", "throw error;");
        test.result(result)
            .assert_no_lint("no-placeholder-implementation");
    }
}
