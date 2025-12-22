use destack_ast::{self as ast, LetKind};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `var` declarations.
    ///
    /// Use `const` for values that don't change and `let` for mutable bindings.
    /// The `var` keyword is a legacy syntax; prefer `let` or `const`.
    #[lint(
        id = "no-var",
        code = "LY012",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoVar,
    "Disallow var declarations"
}

impl LintRule for NoVar {
    fn meta(&self) -> &'static crate::LintMeta {
        NoVar::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            if let ast::Expression::Let {
                kind: LetKind::Var, ..
            } = expr
            {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // make fix: replace var with let
                let expression_span = ctx.tree.get_span(node_id);
                let expr_text = ctx.get_span_text(expression_span);
                let replacement = expr_text
                    .strip_prefix("var")
                    .map(|rest| format!("let{rest}"))
                    .unwrap_or_else(|| expr_text.to_string());
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Replace `var` with `let`").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_VAR.id,
                        NO_VAR.code,
                        NO_VAR.category,
                        severity,
                        "unexpected `var` declaration",
                        ctx.module.file_id,
                        expression_span,
                    )
                    .with_label("use `let` or `const` instead")
                    .with_fix(fix),
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
    fn test_detects_var() {
        let test = TestProgram::for_rule_without_builtins(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 1
"#,
        );
        test.result(result).assert_lint("no-var");
    }

    #[test]
    fn test_allows_let() {
        let test = TestProgram::for_rule_without_builtins(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1
"#,
        );
        test.result(result).assert_no_lint("no-var");
    }

    #[test]
    fn test_allows_const() {
        let test = TestProgram::for_rule_without_builtins(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 1
"#,
        );
        test.result(result).assert_no_lint("no-var");
    }

    #[test]
    fn test_detects_var_in_for() {
        let test = TestProgram::for_rule_without_builtins(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (var i = 0; i < 10; i++) {
    console.log(i)
}
"#,
        );
        test.result(result).assert_lint("no-var");
    }

    #[test]
    fn test_allows_let_in_for() {
        let test = TestProgram::for_rule_without_builtins(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 0; i < 10; i++) {
    console.log(i)
}
"#,
        );
        test.result(result).assert_no_lint("no-var");
    }

    #[test]
    fn test_fix_var_to_let() {
        let test = TestProgram::for_rule_without_builtins(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
var x = 1
"#,
        );
        test.result(result).assert_lint("no-var").assert_safe_fixed(
            r#"
let x = 1;
"#,
        );
    }

    #[test]
    fn test_fix_var_in_for() {
        let test = TestProgram::for_rule_without_builtins(NoVar);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (var i = 0; i < 10; i++) { x() }
"#,
        );
        test.result(result).assert_lint("no-var").assert_safe_fixed(
            r#"
for (let i = 0; i < 10; i++) { x() }
"#,
        );
    }
}
