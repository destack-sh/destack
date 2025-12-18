use destack_ast::{self as ast, ForEachKind, LocalNodeId};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

/// Check if the expression is an if statement, unwrapping Statement wrapper if needed.
fn is_if_expression(ctx: &LintModuleAstContext<'_>, expr_id: LocalNodeId<ast::Expression>) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::If { .. } => true,
        ast::Expression::Statement(inner_id) => is_if_expression(ctx, *inner_id),
        _ => false,
    }
}

declare_lint! {
    /// Require guard in for-in loops.
    ///
    /// For-in loops iterate over all enumerable properties including inherited ones.
    /// Use hasOwnProperty or Object.hasOwn to guard against inherited properties.
    #[lint(
        id = "guard-for-in",
        code = "LD005",
        category = Pedantic,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub GuardForIn,
    "Require guard in for-in loops"
}

impl LintRule for GuardForIn {
    fn meta(&self) -> &'static crate::LintMeta {
        GuardForIn::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // only check for-in loops
            let ast::Expression::ForEach {
                kind: ForEachKind::In,
                body,
                ..
            } = expression
            else {
                continue;
            };

            let block = ctx.tree.get(*body);

            // empty body is fine (though weird)
            if block.expressions.is_empty() {
                continue;
            }

            // check if first expression is an if statement (guard pattern)
            let first_expr_id = block.expressions[0];
            let has_guard = is_if_expression(ctx, first_expr_id);
            if !has_guard {
                ctx.report(
                    LintDiagnostic::new(
                        GUARD_FOR_IN.id,
                        GUARD_FOR_IN.code,
                        GUARD_FOR_IN.category,
                        severity,
                        "for-in loop should have a hasOwnProperty guard",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add if (obj.hasOwnProperty(key)) guard"),
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
    fn test_unguarded_for_in_detected() {
        let test = TestProgram::for_rule(GuardForIn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {
        console.log(key)
    }
}
"#,
        );
        test.result(result).assert_lint("guard-for-in");
    }

    #[test]
    fn test_guarded_for_in_allowed() {
        let test = TestProgram::for_rule(GuardForIn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {
        if (obj.hasOwnProperty(key)) {
            console.log(key)
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_for_of_not_affected() {
        let test = TestProgram::for_rule(GuardForIn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(arr: int32[]) {
    for (const item of arr) {
        console.log(item)
    }
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_empty_for_in_allowed() {
        let test = TestProgram::for_rule(GuardForIn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {}
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }
}
