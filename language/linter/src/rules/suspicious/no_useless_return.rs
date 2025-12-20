use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow redundant return statements at the end of functions.
    ///
    /// A `return` statement with no value at the end of a function is redundant
    /// since the function would return anyway.
    #[lint(
        id = "no-useless-return",
        code = "LU004",
        category = Suspicious,
        level = Ast,
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessReturn,
    "Disallow useless return statements"
}

impl LintRule for NoUselessReturn {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessReturn::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let ast::Declaration::Function { body, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            let Some(body_id) = body else {
                continue;
            };

            // check if the last statement is a bare return
            if let Some(return_id) = get_trailing_bare_return(ctx, *body_id) {
                let severity = ctx.get_effective_severity(meta, return_id);
                if !severity.is_enabled() {
                    continue;
                }

                let return_span = ctx.tree.get_span(return_id);
                let edits = ctx.edit_builder().delete(return_span).into_edits();
                let fix = LintFix::safe("Remove useless return").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_RETURN.id,
                        NO_USELESS_RETURN.code,
                        NO_USELESS_RETURN.category,
                        severity,
                        "useless return statement",
                        ctx.module.file_id,
                        return_span,
                    )
                    .with_label("this return is unnecessary")
                    .with_fix(fix),
                );
            }
        }
    }
}

/// Get a trailing bare return statement from a function body.
fn get_trailing_bare_return(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            if let Some(last_id) = block.expressions.last() {
                return get_trailing_bare_return(ctx, *last_id);
            }
            None
        }
        ast::Expression::Return { value } => {
            if value.is_none() {
                Some(expr_id)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_bare_return_at_end() {
        let test = TestProgram::for_rule_without_builtins(NoUselessReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    bar()
    return
}
"#,
        );
        test.result(result).assert_lint("no-useless-return");
    }

    #[test]
    fn test_allows_return_with_value() {
        let test = TestProgram::for_rule_without_builtins(NoUselessReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    return 42;
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-return");
    }

    #[test]
    fn test_allows_no_return() {
        let test = TestProgram::for_rule_without_builtins(NoUselessReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    console.log("hello");
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-return");
    }

    #[test]
    fn test_allows_early_return() {
        // early return is not useless
        let test = TestProgram::for_rule_without_builtins(NoUselessReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: boolean) {
    if (x) {
        return;
    }
    console.log("continuing");
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-return");
    }

    #[test]
    fn test_fix_removes_useless_return() {
        let test = TestProgram::for_rule_without_builtins(NoUselessReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    bar()
    return
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-return")
            .assert_safe_fixed(
                r#"
function foo() {
    bar()
}
"#,
            );
    }
}
