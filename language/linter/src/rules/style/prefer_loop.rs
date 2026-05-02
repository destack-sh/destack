use crate::LintMeta;
use destack_ast::{self as ast, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer `loop` over `while(true)` or `for(;;)`.
    ///
    /// Use the explicit `loop` keyword for infinite loops instead of
    /// `while(true)`, `while(1)`, or `for(;;)` patterns.
    #[lint(
        id = "prefer-loop",
        code = "LY043",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferLoop,
    "Prefer explicit `loop` for infinite loops"
}

impl LintRule for PreferLoop {
    fn meta(&self) -> &'static LintMeta {
        PreferLoop::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let body_id = match expression {
                // while (true) or while (1)
                ast::Expression::While {
                    condition, body, ..
                } => {
                    if is_always_true(ctx.tree, *condition) {
                        Some(*body)
                    } else {
                        None
                    }
                }
                // for (;;)
                ast::Expression::For {
                    initialization,
                    condition,
                    increment,
                    body,
                    ..
                } => {
                    if initialization.is_none() && condition.is_none() && increment.is_none() {
                        Some(*body)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let Some(body_id) = body_id else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // make fix: replace with loop
            let span = ctx.tree.get_span(node_id);
            let body_span = ctx.tree.get_span(body_id);
            let body_text = ctx.get_span_text(body_span);
            let replacement = format!("loop {body_text}");
            let edits = ctx.edit_builder().replace(span, replacement).into_edits();
            let fix = LintFix::safe("Replace with `loop`").with_edits(edits);

            ctx.report(
                LintReport::new(
                    PREFER_LOOP.id,
                    PREFER_LOOP.code,
                    PREFER_LOOP.category,
                    severity,
                    "use `loop` instead of infinite loop pattern",
                    span,
                )
                .label("replace with `loop { ... }`")
                .fix(fix),
            );
        }
    }
}

/// Check if an expression is always truthy (true or 1).
fn is_always_true(tree: &ast::Tree, expression_id: ast::LocalNodeId<ast::Expression>) -> bool {
    let expression = tree.get(expression_id);

    // unwrap parentheses
    if let ast::Expression::Parenthesized { expression } = expression {
        return is_always_true(tree, *expression);
    }

    matches!(
        expression,
        ast::Expression::ScalarLiteral(ScalarLiteral::Boolean(true))
            | ast::Expression::ScalarLiteral(ScalarLiteral::Integer(1))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_while_true() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        let result = test.lint_ast(
            "prefer_loop/test_detects_while_true.ds",
            r#"
while (true) {
    break
}
"#,
        );
        test.result(result).assert_lint("prefer-loop");
    }

    #[test]
    fn test_detects_while_one() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        let result = test.lint_ast(
            "prefer_loop/test_detects_while_one.ds",
            r#"
while (1) {
    break
}
"#,
        );
        test.result(result).assert_lint("prefer-loop");
    }

    #[test]
    fn test_detects_for_empty() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        let result = test.lint_ast(
            "prefer_loop/test_detects_for_empty.ds",
            r#"
for (;;) {
    break
}
"#,
        );
        test.result(result).assert_lint("prefer-loop");
    }

    #[test]
    fn test_allows_loop() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        let result = test.lint_ast(
            "prefer_loop/test_allows_loop.ds",
            r#"
loop {
    break
}
"#,
        );
        test.result(result).assert_no_lint("prefer-loop");
    }

    #[test]
    fn test_allows_while_condition() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        let result = test.lint_ast(
            "prefer_loop/test_allows_while_condition.ds",
            r#"
while (x > 0) {
    x = x - 1
}
"#,
        );
        test.result(result).assert_no_lint("prefer-loop");
    }

    #[test]
    fn test_allows_for_with_condition() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        let result = test.lint_ast(
            "prefer_loop/test_allows_for_with_condition.ds",
            r#"
for (let i = 0; i < 10; i++) {
    console.log(i)
}
"#,
        );
        test.result(result).assert_no_lint("prefer-loop");
    }

    #[test]
    fn test_allows_for_with_partial() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        // for loop with just initialization is not infinite
        let result = test.lint_ast(
            "prefer_loop/test_allows_for_with_partial.ds",
            r#"
for (let i = 0;;) {
    if (i > 10) break
    i++
}
"#,
        );
        test.result(result).assert_no_lint("prefer-loop");
    }

    #[test]
    fn test_fix_while_true() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        let result = test.lint_ast(
            "prefer_loop/test_fix_while_true.ds",
            r#"
while (true) {
    break
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-loop")
            .assert_safe_fixed(
                r#"
loop {
    break
}
"#,
            );
    }

    #[test]
    fn test_fix_for_empty() {
        let test = TestProgram::for_rule_without_prelude(PreferLoop);
        let result = test.lint_ast(
            "prefer_loop/test_fix_for_empty.ds",
            r#"
for (;;) {
    break
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-loop")
            .assert_safe_fixed(
                r#"
loop {
    break
}
"#,
            );
    }
}
