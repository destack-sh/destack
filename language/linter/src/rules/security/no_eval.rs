use destack_ast::Expression;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow the use of `eval()`.
    ///
    /// `eval()` executes arbitrary code and poses a significant security risk.
    /// It can expose your application to injection attacks and makes code
    /// harder to analyze and optimize. Use safer alternatives like `JSON.parse()`
    /// for data or proper parsing libraries for expressions.
    #[lint(
        id = "no-eval",
        code = "LS001",
        category = Security,
        level = Ast
    )]
    pub NoEval,
    "Disallow eval()"
}

impl LintRule for NoEval {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEval::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<Expression>() {
            let Expression::Call { left, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            let callee = ctx.tree.get(*left);

            // dotted names parse as Path at AST level, not Member
            let Expression::Path { path, .. } = callee else {
                continue;
            };

            let segments = &path.segments;

            // direct eval() call
            if segments.len() == 1 {
                let name = ctx.strings.get(segments[0]);
                if &*name == "eval" {
                    report_eval(ctx, severity, node_id);
                }
                continue;
            }

            // window.eval() or globalThis.eval()
            if segments.len() == 2 {
                let first = ctx.strings.get(segments[0]);
                let second = ctx.strings.get(segments[1]);
                let is_global = &*first == "window" || &*first == "globalThis";
                if &*second == "eval" && is_global {
                    report_eval(ctx, severity, node_id);
                }
            }

            // NOTE: obj.eval() is allowed, only global eval contexts are flagged
        }
    }
}

fn report_eval(
    ctx: &mut LintModuleAstContext<'_>,
    severity: LintSeverity,
    node_id: destack_ast::LocalNodeId<Expression>,
) {
    ctx.report(
        LintDiagnostic::new(
            NO_EVAL.id,
            NO_EVAL.code,
            NO_EVAL.category,
            severity,
            "`eval()` is a security risk",
            ctx.module.file_id,
            ctx.tree.get_span(node_id),
        )
        .with_label("avoid using eval"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_eval_call() {
        let test = TestProgram::for_rule(NoEval);
        let result = test.lint_ast(
            "test.ds",
            r#"
eval("console.log('hello')");
"#,
        );
        test.result(result).assert_lint("no-eval");
    }

    #[test]
    fn test_detects_eval_with_variable() {
        let test = TestProgram::for_rule(NoEval);
        let result = test.lint_ast(
            "test.ds",
            r#"
const code = "1 + 2";
eval(code);
"#,
        );
        test.result(result).assert_lint("no-eval");
    }

    #[test]
    fn test_detects_window_eval() {
        let test = TestProgram::for_rule(NoEval);
        let result = test.lint_ast("test.ds", "window.eval(code);");
        test.result(result).assert_lint("no-eval");
    }

    #[test]
    fn test_detects_globalthis_eval() {
        let test = TestProgram::for_rule(NoEval);
        let result = test.lint_ast("test.ds", "globalThis.eval(code);");
        test.result(result).assert_lint("no-eval");
    }

    #[test]
    fn test_allows_eval_as_variable_name() {
        let test = TestProgram::for_rule(NoEval);
        let result = test.lint_ast(
            "test.ds",
            r#"
const eval = (x: int32) => x * 2;
"#,
        );
        // this is just a variable declaration, not a call
        test.result(result).assert_no_lint("no-eval");
    }

    #[test]
    fn test_allows_other_functions() {
        let test = TestProgram::for_rule(NoEval);
        let result = test.lint_ast("test.ds", "console.log(x); JSON.parse(y); parseInt(z);");
        test.result(result).assert_no_lint("no-eval");
    }

    #[test]
    fn test_allows_evaluate_function() {
        let test = TestProgram::for_rule(NoEval);
        let result = test.lint_ast("test.ds", "evaluate(expression);");
        test.result(result).assert_no_lint("no-eval");
    }

    #[test]
    fn test_allows_object_eval_method() {
        // obj.eval() is allowed, only global contexts are flagged
        let test = TestProgram::for_rule(NoEval);
        let result = test.lint_ast("test.ds", "myObject.eval(code);");
        test.result(result).assert_no_lint("no-eval");
    }
}
