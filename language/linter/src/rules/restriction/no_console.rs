use destack_ast::Expression;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow the use of `console`.
    ///
    /// Console statements are useful for debugging but should not be committed
    /// to production code. Use a proper logging library instead.
    #[lint(
        id = "no-console",
        code = "LR001",
        category = Restriction,
        level = Ast
    )]
    pub NoConsole,
    "Disallow console statements"
}

impl LintRule for NoConsole {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConsole::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<Expression>() {
            // In Destack, `console.log(...)` is parsed as a Call with a Path left.
            // The Path has segments: ["console", "log"]
            let Expression::Call { left, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // Check if the callee is a path starting with "console"
            let left_expr = ctx.tree.get(*left);
            let method_name = match left_expr {
                Expression::Path { path, .. } => {
                    if path.segments.len() >= 2 {
                        let first_segment = ctx.strings.get(path.segments[0]);
                        if &*first_segment == "console" {
                            Some(ctx.strings.get(path.segments[1]).to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let Some(method_name_str) = method_name else {
                continue;
            };

            ctx.report(
                LintDiagnostic::new(
                    NO_CONSOLE.id,
                    NO_CONSOLE.code,
                    NO_CONSOLE.category,
                    severity,
                    format!("`console.{method_name_str}` should not be used in production"),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("remove or replace with proper logging"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_console_log() {
        let test = TestProgram::for_rule(NoConsole);
        let result = test.lint_ast(
            "test.ds",
            r#"
console.log("hello");
"#,
        );
        test.result(result).assert_lint("no-console");
    }

    #[test]
    fn test_detects_console_error() {
        let test = TestProgram::for_rule(NoConsole);
        let result = test.lint_ast(
            "test.ds",
            r#"
console.error("something went wrong");
"#,
        );
        test.result(result).assert_lint("no-console");
    }

    #[test]
    fn test_detects_console_warn() {
        let test = TestProgram::for_rule(NoConsole);
        let result = test.lint_ast(
            "test.ds",
            r#"
console.warn("deprecated");
"#,
        );
        test.result(result).assert_lint("no-console");
    }

    #[test]
    fn test_detects_console_info() {
        let test = TestProgram::for_rule(NoConsole);
        let result = test.lint_ast(
            "test.ds",
            r#"
console.info("info message");
"#,
        );
        test.result(result).assert_lint("no-console");
    }

    #[test]
    fn test_detects_console_debug() {
        let test = TestProgram::for_rule(NoConsole);
        let result = test.lint_ast(
            "test.ds",
            r#"
console.debug("debug message");
"#,
        );
        test.result(result).assert_lint("no-console");
    }

    #[test]
    fn test_allows_other_objects() {
        let test = TestProgram::for_rule(NoConsole);
        let result = test.lint_ast(
            "test.ds",
            r#"
logger.log("hello");
output.error("error");
"#,
        );
        test.result(result).assert_no_lint("no-console");
    }

    #[test]
    fn test_allows_console_variable() {
        let test = TestProgram::for_rule(NoConsole);
        let result = test.lint_ast(
            "test.ds",
            r#"
const console = getConsole();
"#,
        );
        // just a variable declaration, not a member access
        test.result(result).assert_no_lint("no-console");
    }
}
