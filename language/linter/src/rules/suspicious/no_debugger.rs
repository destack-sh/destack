use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_statement_ancestor;
use crate::{LintAstContext, LintDiagnostic, LintFix, LintMeta, LintRule, declare_lint};

declare_lint! {
    /// Disallow debugger statements in production code.
    ///
    /// Debugger statements should not be committed to production code
    /// as they can cause the program to pause unexpectedly.
    #[lint(
        id = "no-debugger",
        code = "LU008",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoDebugger,
    "Disallow debugger statements"
}

impl LintRule for NoDebugger {
    fn meta(&self) -> &'static LintMeta {
        NoDebugger::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect expression nodes for debugger usage
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // keep only debugger expressions
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::Debugger) {
                continue;
            }

            // resolve the effective lint severity at this node
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build one base diagnostic before optional fix attachment
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

            // report without fix payload when fixes are disabled
            if !ctx.compute_fixes {
                ctx.report(diagnostic);
                continue;
            }

            // prefer deleting the full enclosing statement as a safe fix
            if let Some(statement_expression_id) =
                expression_statement_ancestor(ctx.tree, ctx.parents, node_id)
            {
                let statement_span = ctx.tree.get_span(statement_expression_id);
                let diagnostic = diagnostic
                    .with_fix(LintFix::safe("Remove debugger statement").delete(statement_span));
                ctx.report(diagnostic);
                continue;
            }

            // fall back to deleting only the expression span as an unsafe fix
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
        let test = TestProgram::for_rule_without_prelude(NoDebugger);
        let result = test.lint_ast(
            "no_debugger/test_detects_debugger_statement.ds",
            r#"
debugger;
"#,
        );
        test.result(result).assert_lint("no-debugger");
    }

    #[test]
    fn test_detects_debugger_expression() {
        let test = TestProgram::for_rule_without_prelude(NoDebugger);
        let result = test.lint_ast(
            "no_debugger/test_detects_debugger_expression.ds",
            r#"
let x = debugger;
"#,
        );
        test.result(result).assert_lint("no-debugger");
    }

    #[test]
    fn test_detects_multiple_debuggers() {
        let test = TestProgram::for_rule_without_prelude(NoDebugger);
        let result = test.lint_ast(
            "no_debugger/test_detects_multiple_debuggers.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoDebugger);
        let result = test.lint_ast(
            "no_debugger/test_no_debugger_clean_code.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoDebugger);
        let result = test.lint_ast(
            "no_debugger/test_fix_removes_debugger_statement.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoDebugger);
        let result = test.lint_ast(
            "no_debugger/test_fix_preserves_surrounding_code.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoDebugger);
        let result = test.lint_ast(
            "no_debugger/test_fix_without_semicolon.ds",
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
