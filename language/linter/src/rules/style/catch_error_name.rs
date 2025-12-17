use destack_ast::{self as ast, Pattern};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce a specific name for caught errors.
    ///
    /// Consistent naming of caught errors improves code readability.
    /// Configure the expected name via `catch_error_name` option (default: "error").
    #[lint(
        id = "catch-error-name",
        code = "LY018",
        category = Style,
        level = Ast
    )]
    pub CatchErrorName,
    "Enforce consistent catch error naming"
}

impl LintRule for CatchErrorName {
    fn meta(&self) -> &'static crate::LintMeta {
        CatchErrorName::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let expected_name = &ctx.options.catch_error_name;

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for try expressions with catch patterns
            let ast::Expression::Try {
                catch_pattern: Some(pattern_id),
                ..
            } = expr
            else {
                continue;
            };

            let pattern = ctx.tree.get(*pattern_id);

            // extract the binding name from the pattern
            let actual_name = match pattern {
                Pattern::Binding { name, .. } => Some(*name),
                _ => None,
            };

            if let Some(name_id) = actual_name {
                let actual = ctx.strings.get(name_id);
                if actual.as_ref() != expected_name {
                    ctx.report(
                        LintDiagnostic::new(
                            CATCH_ERROR_NAME.id,
                            CATCH_ERROR_NAME.code,
                            CATCH_ERROR_NAME.category,
                            severity,
                            format!(
                                "catch error should be named `{}`, not `{}`",
                                expected_name,
                                actual.as_ref()
                            ),
                            ctx.module.file_id,
                            ctx.tree.get_span(*pattern_id),
                        )
                        .with_label(format!("rename to `{expected_name}`")),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_wrong_error_name() {
        let test = TestProgram::for_rule(CatchErrorName);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    doSomething()
} catch (e) {
    console.log(e)
}
"#,
        );
        test.result(result).assert_lint("catch-error-name");
    }

    #[test]
    fn test_allows_correct_name() {
        let test = TestProgram::for_rule(CatchErrorName);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    doSomething()
} catch (error) {
    console.log(error)
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    #[test]
    fn test_allows_try_without_catch() {
        let test = TestProgram::for_rule(CatchErrorName);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    doSomething()
} finally {
    cleanup()
}
"#,
        );
        test.result(result).assert_no_lint("catch-error-name");
    }

    #[test]
    fn test_detects_err_name() {
        let test = TestProgram::for_rule(CatchErrorName);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    fetch()
} catch (err) {
    console.error(err)
}
"#,
        );
        test.result(result).assert_lint("catch-error-name");
    }
}
