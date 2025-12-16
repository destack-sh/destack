use destack_ast as ast;
use destack_workspace::LintSeverity;

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

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::Debugger) {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            let diagnostic = LintDiagnostic::new(
                NO_DEBUGGER.id,
                NO_DEBUGGER.code,
                NO_DEBUGGER.category,
                severity,
                "debugger statement is not allowed",
                ctx.module.file_id,
                span,
            )
            .with_label("remove this debugger statement");

            // check if parent is Statement (i.e., `debugger;` as a standalone statement)
            // if so, delete the whole statement span safely (includes semicolon)
            if let Some(parent_id) = ctx.parents.get(node_id)
                && ctx.tree.get_node_type(parent_id) == ast::NodeType::Expression
            {
                let parent_expr_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
                let parent_expr = ctx.tree.get(parent_expr_id);
                if matches!(parent_expr, ast::Expression::Statement(_)) {
                    let parent_span = ctx.tree.get_span(parent_expr_id);
                    let diagnostic = diagnostic
                        .with_fix(LintFix::safe("Remove debugger statement").delete(parent_span));
                    ctx.report(diagnostic);
                    continue;
                }
            }
            // not in statement position: offer unsafe fix only
            let diagnostic =
                diagnostic.with_fix(LintFix::r#unsafe("Remove debugger expression").delete(span));
            ctx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_debugger_statement() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast(
            "test.ds",
            r#"
debugger;
"#,
        );
        test.result(result).assert_lint("no-debugger");
    }

    #[test]
    fn test_detects_debugger_expression() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = debugger;
"#,
        );
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

    #[test]
    fn test_fix_removes_debugger_statement() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast(
            "test.ds",
            r#"
debugger;
"#,
        );
        test.result(result)
            .assert_lint("no-debugger")
            .assert_has_fix("no-debugger")
            .assert_safe_fixed(r#""#);
    }

    #[test]
    fn test_fix_preserves_surrounding_code() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
debugger;
let y = 2;
"#,
        );
        test.result(result)
            .assert_lint("no-debugger")
            .assert_safe_fixed(
                r#"
let x = 1;

let y = 2;
"#,
            );
    }

    #[test]
    fn test_fix_without_semicolon() {
        let test = TestProgram::for_rule(NoDebugger);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = debugger;
"#,
        );
        test.result(result)
            .assert_lint("no-debugger")
            .assert_safe_fixed(
                r#"
let x = debugger;
"#,
            )
            .assert_unsafe_fixed(
                r#"
let x = ;
"#,
            );
    }
}
