use destack_ast::Expression;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow debugger statements in production code.
    ///
    /// Debugger statements should not be committed to production code
    /// as they can cause the program to pause unexpectedly.
    #[lint(
        id = "no-debugger",
        code = "LC001",
        category = Suspicious,
        level = Ast,
        fixable
    )]
    pub NoDebugger,
    "Disallow debugger statements"
}

impl LintRule for NoDebugger {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDebugger::meta()
    }

    fn check_module_ast(&self, context: &mut LintModuleAstContext) {
        let file_id = context.file_id;
        let severity = context.get_severity(NoDebugger::meta());

        context.for_each::<Expression, _>(|expression, span| {
            if matches!(expression, Expression::Debugger) {
                Some(
                    LintDiagnostic::new(
                        NO_DEBUGGER.id,
                        NO_DEBUGGER.code,
                        NO_DEBUGGER.category,
                        severity,
                        "debugger statement is not allowed",
                        file_id,
                        span,
                    )
                    .with_label("remove this debugger statement")
                    .with_fix(LintFix::safe("Remove debugger statement").delete(span)),
                )
            } else {
                None
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_debugger_statement() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast("test.ds", "debugger;");
        test.result(result).assert_lint("no-debugger");
    }

    #[test]
    fn test_detects_debugger_expression() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast("test.ds", "let x = debugger;");
        test.result(result).assert_lint("no-debugger");
    }

    #[test]
    fn test_detects_multiple_debuggers() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast(
            "test.ds",
            r#"
debugger;
function foo() {
    debugger;
}
debugger;
"#,
        );
        test.result(result).assert_lint_count("no-debugger", 3);
    }

    #[test]
    fn test_no_debugger_clean_code() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
function foo() {
    return x + 1;
}
"#,
        );
        test.result(result).assert_no_lint("no-debugger");
    }
}
