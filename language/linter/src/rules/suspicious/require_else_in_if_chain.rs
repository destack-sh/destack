use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_is_else_if_branch;
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Require final else in if-else-if chains.
    ///
    /// An if-else-if chain without a final else clause may indicate
    /// missing logic for unhandled cases.
    #[lint(
        id = "require-else-in-if-chain",
        code = "LU043",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireElseInIfChain,
    "Require else in if chains"
}

impl LintRule for RequireElseInIfChain {
    fn meta(&self) -> &'static LintMeta {
        RequireElseInIfChain::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // only lint the chain head to avoid duplicate diagnostics
            if expression_is_else_if_branch(ctx.tree, ctx.parents, node_id) {
                continue;
            }

            let expression = ctx.tree.get(node_id);
            let ast::Expression::If {
                else_expression: Some(else_expr),
                ..
            } = expression
            else {
                continue;
            };

            // find the terminal else-if in this chain when it has no fallback else
            let Some(terminal_else_if_id) = find_terminal_else_if_without_fallback(ctx, *else_expr)
            else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                REQUIRE_ELSE_IN_IF_CHAIN.id,
                REQUIRE_ELSE_IN_IF_CHAIN.code,
                REQUIRE_ELSE_IN_IF_CHAIN.category,
                severity,
                "if-else-if chain lacks final else clause",
                ctx.module.file_id,
                ctx.tree.get_span(terminal_else_if_id),
            )
            .with_label("add final else clause");
            if ctx.compute_fixes
                && let Some(fix) = require_else_in_if_chain_fix(ctx, terminal_else_if_id)
            {
                diagnostic = diagnostic.with_fix(fix);
            }
            ctx.report(diagnostic);
        }
    }
}

/// Follow else wrappers and return the terminal else-if without fallback else.
fn find_terminal_else_if_without_fallback(
    ctx: &LintAstContext<'_>,
    else_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let mut current_id = else_expression_id;

    loop {
        let current_expression = ctx.tree.get(current_id);

        let ast::Expression::If {
            else_expression, ..
        } = current_expression
        else {
            return None;
        };

        // a missing else means this is the terminal else-if
        let Some(next_else_id) = else_expression else {
            return Some(current_id);
        };
        current_id = *next_else_id;
    }
}

/// Build an unsafe fix by appending an empty final else branch.
fn require_else_in_if_chain_fix(
    ctx: &LintAstContext<'_>,
    terminal_else_if_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let terminal_expression = ctx.tree.get(terminal_else_if_id);
    if !matches!(
        terminal_expression,
        ast::Expression::If {
            else_expression: None,
            ..
        }
    ) {
        return None;
    }

    let terminal_span = ctx.tree.get_span(terminal_else_if_id);
    let terminal_text = ctx.get_span_text(terminal_span);
    let replacement = format!("{terminal_text} else {{}}");
    let edits = ctx
        .edit_builder()
        .replace(terminal_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Add a final else branch").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_if_else_if_without_else_detected() {
        let test = TestProgram::for_rule_without_prelude(RequireElseInIfChain);
        let result = test.lint_ast(
            "require_else_in_if_chain/test_if_else_if_without_else_detected.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    } else if (x < 0) {
        console.log("negative")
    }
}
"#,
        );
        test.result(result)
            .assert_lint("require-else-in-if-chain")
            .assert_unsafe_fixed(
                r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    } else if (x < 0) {
        console.log("negative")
    } else {
    }
}
"#,
            );
    }

    #[test]
    fn test_if_else_if_else_allowed() {
        let test = TestProgram::for_rule_without_prelude(RequireElseInIfChain);
        let result = test.lint_ast(
            "require_else_in_if_chain/test_if_else_if_else_allowed.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    } else if (x < 0) {
        console.log("negative")
    } else {
        console.log("zero")
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("require-else-in-if-chain");
    }

    #[test]
    fn test_simple_if_allowed() {
        let test = TestProgram::for_rule_without_prelude(RequireElseInIfChain);
        let result = test.lint_ast(
            "require_else_in_if_chain/test_simple_if_allowed.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("require-else-in-if-chain");
    }

    #[test]
    fn test_simple_if_else_allowed() {
        let test = TestProgram::for_rule_without_prelude(RequireElseInIfChain);
        let result = test.lint_ast(
            "require_else_in_if_chain/test_simple_if_else_allowed.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        console.log("positive")
    } else {
        console.log("not positive")
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("require-else-in-if-chain");
    }

    #[test]
    fn test_reports_chain_once() {
        let test = TestProgram::for_rule_without_prelude(RequireElseInIfChain);
        let result = test.lint_ast(
            "require_else_in_if_chain/test_reports_chain_once.ds",
            r#"
if (x == 0) {
    a()
} else if (x == 1) {
    b()
} else if (x == 2) {
    c()
}
"#,
        );
        test.result(result)
            .assert_lint_count("require-else-in-if-chain", 1);
    }

    #[test]
    fn test_allows_else_block_with_nested_if() {
        let test = TestProgram::for_rule_without_prelude(RequireElseInIfChain);
        let result = test.lint_ast(
            "require_else_in_if_chain/test_allows_else_block_with_nested_if.ds",
            r#"
if (x > 0) {
    a()
} else {
    if (x < 0) {
        b()
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("require-else-in-if-chain");
    }
}
