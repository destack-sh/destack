use destack_ast::Expression;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require radix parameter in `parseInt()`.
    ///
    /// When using `parseInt()`, always specify the radix parameter to avoid
    /// unexpected behavior. Without it, strings starting with "0" might be
    /// interpreted as octal in older environments.
    ///
    /// Bad: `parseInt("10")`
    /// Good: `parseInt("10", 10)`
    #[lint(
        id = "radix",
        code = "LD001",
        category = Pedantic,
        level = Ast
    )]
    pub Radix,
    "Require radix in parseInt"
}

impl LintRule for Radix {
    fn meta(&self) -> &'static crate::LintMeta {
        Radix::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<Expression>() {
            let Expression::Call {
                left,
                dynamic_arguments,
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // check if it's a call to parseInt
            let callee = ctx.tree.get(*left);
            let is_parse_int = match callee {
                Expression::Path { path, .. } => {
                    path.segments.len() == 1 && {
                        let name = ctx.strings.get(path.segments[0]);
                        &*name == "parseInt"
                    }
                }
                Expression::Member { name, .. } => {
                    let member_name = ctx.strings.get(*name);
                    &*member_name == "parseInt"
                }
                _ => false,
            };

            if !is_parse_int {
                continue;
            }

            // check if radix argument is missing (only 1 argument)
            if dynamic_arguments.len() == 1 {
                let span = ctx.tree.get_span(node_id);
                ctx.report(
                    LintDiagnostic::new(
                        RADIX.id,
                        RADIX.code,
                        RADIX.category,
                        severity,
                        "`parseInt()` missing radix parameter",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("add radix parameter (e.g., 10 for decimal)"),
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
    fn test_detects_missing_radix() {
        let test = TestProgram::for_rule(Radix);
        let result = test.lint_ast(
            "test.ds",
            r#"
const num = parseInt("42");
"#,
        );
        test.result(result).assert_lint("radix");
    }

    #[test]
    fn test_detects_missing_radix_with_variable() {
        let test = TestProgram::for_rule(Radix);
        let result = test.lint_ast(
            "test.ds",
            r#"
const str = "100";
const num = parseInt(str);
"#,
        );
        test.result(result).assert_lint("radix");
    }

    #[test]
    fn test_allows_with_radix() {
        let test = TestProgram::for_rule(Radix);
        let result = test.lint_ast(
            "test.ds",
            r#"
const num = parseInt("42", 10);
"#,
        );
        test.result(result).assert_no_lint("radix");
    }

    #[test]
    fn test_allows_hex_radix() {
        let test = TestProgram::for_rule(Radix);
        let result = test.lint_ast(
            "test.ds",
            r#"
const num = parseInt("ff", 16);
"#,
        );
        test.result(result).assert_no_lint("radix");
    }

    #[test]
    fn test_allows_binary_radix() {
        let test = TestProgram::for_rule(Radix);
        let result = test.lint_ast(
            "test.ds",
            r#"
const num = parseInt("1010", 2);
"#,
        );
        test.result(result).assert_no_lint("radix");
    }

    #[test]
    fn test_allows_number_parse_int() {
        let test = TestProgram::for_rule(Radix);
        let result = test.lint_ast(
            "test.ds",
            r#"
const num = Number.parseInt("42", 10);
"#,
        );
        test.result(result).assert_no_lint("radix");
    }

    #[test]
    fn test_detects_number_parse_int_missing_radix() {
        let test = TestProgram::for_rule(Radix);
        let result = test.lint_ast(
            "test.ds",
            r#"
const num = Number.parseInt("42");
"#,
        );
        test.result(result).assert_lint("radix");
    }

    #[test]
    fn test_allows_other_functions() {
        let test = TestProgram::for_rule(Radix);
        let result = test.lint_ast(
            "test.ds",
            r#"
const num = parseFloat("3.14");
const str = String(42);
"#,
        );
        test.result(result).assert_no_lint("radix");
    }
}
