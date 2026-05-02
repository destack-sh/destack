use crate::LintMeta;
use destack_ast::{self as ast, ForEachBinding, ForEachDeclarationKind, LetKind};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `var` declarations.
    ///
    /// Use `const` for values that don't change and `let` for mutable bindings.
    /// The `var` keyword is a legacy form; prefer `let` or `const`.
    #[lint(
        id = "no-var",
        code = "LY026",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoVar,
    "Disallow var declarations"
}

impl LintRule for NoVar {
    fn meta(&self) -> &'static LintMeta {
        NoVar::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // direct `var` declarations
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

                let expression_span = ctx.tree.get_span(node_id);
                let mut diagnostic = LintReport::new(
                    NO_VAR.id,
                    NO_VAR.code,
                    NO_VAR.category,
                    severity,
                    "unexpected `var` declaration",
                    expression_span,
                )
                .label("use `let` or `const` instead");

                if ctx.compute_fixes
                    && let Some(fix) = no_var_fix(ctx, expression_span)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }

        // `for (var x of y)` and `for (var x in y)` bindings
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::ForEach { binding, .. } = expression else {
                continue;
            };

            let ForEachBinding::Pattern {
                declaration_kind: Some(ForEachDeclarationKind::Var),
                ..
            } = binding
            else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintReport::new(
                    NO_VAR.id,
                    NO_VAR.code,
                    NO_VAR.category,
                    severity,
                    "unexpected `var` declaration",
                    ctx.tree.get_span(node_id),
                )
                .label("use `let` or `const` instead"),
            );
        }
    }
}

/// Build a safe fix that rewrites one `var` keyword to `let`.
fn no_var_fix(ctx: &LintAstContext<'_>, span: destack_source::Span) -> Option<LintFix> {
    let text = ctx.get_span_text(span);
    let trimmed_text = text.trim_start();
    let leading_whitespace_len = text.len().checked_sub(trimmed_text.len())?;

    if !trimmed_text.starts_with("var") {
        return None;
    }

    let suffix = &trimmed_text["var".len()..];
    if suffix
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }

    let prefix = &text[..leading_whitespace_len];
    let replacement = format!("{prefix}let{suffix}");
    let edits = ctx.edit_builder().replace(span, replacement).into_edits();
    Some(LintFix::safe("Replace `var` with `let`").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_var() {
        let test = TestProgram::for_rule_without_prelude(NoVar);
        let result = test.lint_ast(
            "no_var/test_detects_var.ds",
            r#"
var x = 1
"#,
        );
        test.result(result).assert_lint("no-var");
    }

    #[test]
    fn test_allows_let() {
        let test = TestProgram::for_rule_without_prelude(NoVar);
        let result = test.lint_ast(
            "no_var/test_allows_let.ds",
            r#"
let x = 1
"#,
        );
        test.result(result).assert_no_lint("no-var");
    }

    #[test]
    fn test_allows_const() {
        let test = TestProgram::for_rule_without_prelude(NoVar);
        let result = test.lint_ast(
            "no_var/test_allows_const.ds",
            r#"
const x = 1
"#,
        );
        test.result(result).assert_no_lint("no-var");
    }

    #[test]
    fn test_detects_var_in_for() {
        let test = TestProgram::for_rule_without_prelude(NoVar);
        let result = test.lint_ast(
            "no_var/test_detects_var_in_for.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoVar);
        let result = test.lint_ast(
            "no_var/test_allows_let_in_for.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoVar);
        let result = test.lint_ast(
            "no_var/test_fix_var_to_let.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoVar);
        let result = test.lint_ast(
            "no_var/test_fix_var_in_for.ds",
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

    #[test]
    fn test_detects_var_for_each_binding() {
        let test = TestProgram::for_rule_without_prelude(NoVar);
        let result = test.lint_ast(
            "no_var/test_detects_var_for_each_binding.ds",
            r#"
for (var item of items) {
    sink(item)
}
"#,
        );
        test.result(result).assert_lint("no-var");
    }
}
