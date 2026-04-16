use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_is_unqualified_path_name;
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Disallow catch clauses that only rethrow the caught error.
    ///
    /// A catch clause that only rethrows the original error is useless.
    /// The same behavior can be achieved by removing the try-catch entirely.
    #[lint(
        id = "no-useless-catch",
        code = "LU035",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessCatch,
    "Disallow catch that just rethrows"
}

impl LintRule for NoUselessCatch {
    fn meta(&self) -> &'static LintMeta {
        NoUselessCatch::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Try {
                catch_pattern,
                catch_expression,
                finally_expression,
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // need both a catch pattern and expression
            let (Some(pattern_id), Some(catch_expr_id)) = (catch_pattern, catch_expression) else {
                continue;
            };

            // get the bound name from the catch pattern
            let Some(catch_name) = get_pattern_binding_name(ctx, *pattern_id) else {
                continue;
            };

            // check if the catch expression just throws the caught variable
            if is_throw_of_name(ctx, *catch_expr_id, catch_name) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let has_finally_clause = finally_expression.is_some();
                let diagnostic_span = if has_finally_clause {
                    ctx.tree.get_span(*catch_expr_id)
                } else {
                    ctx.tree.get_span(node_id)
                };

                let diagnostic = LintDiagnostic::new(
                    NO_USELESS_CATCH.id,
                    NO_USELESS_CATCH.code,
                    NO_USELESS_CATCH.category,
                    severity,
                    "useless catch clause",
                    ctx.module.file_id,
                    diagnostic_span,
                )
                .with_label("this catch only rethrows the original error");

                ctx.report(diagnostic);
            }
        }
    }
}

/// Get the binding name from a simple catch pattern.
fn get_pattern_binding_name(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<ast::StringId> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Binding { name, .. } => Some(*name),
        _ => None,
    }
}

/// Check if an expression is `throw <name>` where name matches the given string id.
fn is_throw_of_name(
    ctx: &LintAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    name: ast::StringId,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        // direct throw
        ast::Expression::Throw { value } => {
            expression_is_unqualified_path_name(ctx.tree, *value, name)
        }
        // block with exactly one throw of the caught name
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            block.len() == 1
                && block
                    .first_expression()
                    .is_some_and(|expression_id| is_throw_of_name(ctx, expression_id, name))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_useless_catch_throw() {
        let test = TestProgram::for_rule_without_prelude(NoUselessCatch);
        let result = test.lint_ast(
            "no_useless_catch/test_detects_useless_catch_throw.ds",
            r#"
try {
    foo()
} catch e {
    throw e
}
"#,
        );
        test.result(result).assert_lint("no-useless-catch");
    }

    #[test]
    fn test_detects_useless_catch_block_throw() {
        let test = TestProgram::for_rule_without_prelude(NoUselessCatch);
        let result = test.lint_ast(
            "no_useless_catch/test_detects_useless_catch_block_throw.ds",
            r#"
try {
    foo()
} catch err {
    throw err
}
"#,
        );
        test.result(result).assert_lint("no-useless-catch");
    }

    #[test]
    fn test_allows_catch_with_logging() {
        let test = TestProgram::for_rule_without_prelude(NoUselessCatch);
        let result = test.lint_ast(
            "no_useless_catch/test_allows_catch_with_logging.ds",
            r#"
try {
    riskyOperation();
} catch e {
    console.log(e);
    throw e;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-catch");
    }

    #[test]
    fn test_allows_catch_with_different_throw() {
        let test = TestProgram::for_rule_without_prelude(NoUselessCatch);
        let result = test.lint_ast(
            "no_useless_catch/test_allows_catch_with_different_throw.ds",
            r#"
try {
    riskyOperation();
} catch e {
    throw new Error("wrapped");
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-catch");
    }

    #[test]
    fn test_detects_useless_catch_with_finally_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoUselessCatch);
        let result = test.lint_ast(
            "no_useless_catch/test_detects_useless_catch_with_finally_without_fix.ds",
            r#"
try {
    riskyOperation();
} catch e {
    throw e;
} finally {
    cleanup();
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-catch")
            .assert_has_no_fix("no-useless-catch");
    }

    #[test]
    fn test_allows_catch_with_extra_unreachable_statement() {
        let test = TestProgram::for_rule_without_prelude(NoUselessCatch);
        let result = test.lint_ast(
            "no_useless_catch/test_allows_catch_with_extra_unreachable_statement.ds",
            r#"
try {
    riskyOperation();
} catch e {
    throw e;
    logUnreachable(e);
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-catch");
    }

    #[test]
    fn test_reports_useless_catch_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoUselessCatch);
        let result = test.lint_ast(
            "no_useless_catch/test_reports_useless_catch_without_fix.ds",
            r#"
try {
    foo()
} catch e {
    throw e
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-catch")
            .assert_has_no_fix("no-useless-catch");
    }
}
