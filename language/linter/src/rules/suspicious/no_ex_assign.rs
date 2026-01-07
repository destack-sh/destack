use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow reassigning exceptions in catch clauses.
    ///
    /// Reassigning the exception variable in a catch clause is almost always
    /// a mistake. It loses the original error information and makes debugging
    /// harder.
    #[lint(
        id = "no-ex-assign",
        code = "LU018",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoExAssign,
    "Disallow reassigning exceptions in catch clauses"
}

impl LintRule for NoExAssign {
    fn meta(&self) -> &'static crate::LintMeta {
        NoExAssign::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Try {
                catch_pattern,
                catch_expression,
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // need both pattern and expression
            let (Some(pattern_id), Some(catch_expr_id)) = (catch_pattern, catch_expression) else {
                continue;
            };

            // extract the bound name
            let Some(catch_name) = get_pattern_binding_name(ctx, *pattern_id) else {
                continue;
            };

            // check for assignments in the catch body
            check_assignments_in_expression(ctx, meta, *catch_expr_id, catch_name);
        }
    }
}

/// Extract the binding name from a simple catch pattern.
fn get_pattern_binding_name(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<ast::StringId> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Binding { name, .. } => Some(*name),
        _ => None,
    }
}

/// Check for assignments to the catch variable within an expression.
fn check_assignments_in_expression(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    expr_id: ast::LocalNodeId<ast::Expression>,
    catch_name: ast::StringId,
) {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Assign { left, right, .. } => {
            if is_path_to_name(ctx, *left, catch_name) {
                report_ex_assign(ctx, meta, expr_id);
            }
            check_assignments_in_expression(ctx, meta, *right, catch_name);
        }
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            for nested_id in &block.expressions {
                check_assignments_in_expression(ctx, meta, *nested_id, catch_name);
            }
        }
        ast::Expression::If {
            then_expression,
            else_expression,
            ..
        } => {
            check_assignments_in_expression(ctx, meta, *then_expression, catch_name);
            if let Some(else_id) = else_expression {
                check_assignments_in_expression(ctx, meta, *else_id, catch_name);
            }
        }
        ast::Expression::Parenthesized { expression } => {
            check_assignments_in_expression(ctx, meta, *expression, catch_name);
        }
        ast::Expression::Binary { left, right, .. } => {
            check_assignments_in_expression(ctx, meta, *left, catch_name);
            check_assignments_in_expression(ctx, meta, *right, catch_name);
        }
        ast::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } => {
            check_assignments_in_expression(ctx, meta, *left, catch_name);
            for arg_id in dynamic_arguments {
                let arg = ctx.tree.get(*arg_id);
                if let ast::Argument::Positional { value, .. } = arg {
                    check_assignments_in_expression(ctx, meta, *value, catch_name);
                }
            }
        }
        ast::Expression::Statement(inner_id) => {
            check_assignments_in_expression(ctx, meta, *inner_id, catch_name);
        }
        _ => {}
    }
}

/// Check if expression is a simple path to the given name.
fn is_path_to_name(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
    name: ast::StringId,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Path { path, .. } => path.segments.len() == 1 && path.segments[0] == name,
        ast::Expression::Parenthesized { expression } => is_path_to_name(ctx, *expression, name),
        _ => false,
    }
}

/// Report an exception reassignment diagnostic.
fn report_ex_assign(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    expr_id: ast::LocalNodeId<ast::Expression>,
) {
    let severity = ctx.get_effective_severity(meta, expr_id);
    if !severity.is_enabled() {
        return;
    }

    let span = ctx.tree.get_span(expr_id);
    ctx.report(
        LintDiagnostic::new(
            NO_EX_ASSIGN.id,
            NO_EX_ASSIGN.code,
            NO_EX_ASSIGN.category,
            severity,
            "do not reassign the exception variable",
            ctx.module.file_id,
            span,
        )
        .with_label("this reassignment loses the original error"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_exception_reassignment() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    riskyOperation();
} catch e {
    e = null;
}
"#,
        );
        test.result(result).assert_lint("no-ex-assign");
    }

    #[test]
    fn test_flags_exception_reassignment_with_new() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    riskyOperation();
} catch e {
    e = new Error("replaced");
}
"#,
        );
        test.result(result).assert_lint("no-ex-assign");
    }

    #[test]
    fn test_allows_catch_without_reassignment() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    riskyOperation();
} catch e {
    console.log(e);
    throw e;
}
"#,
        );
        test.result(result).assert_no_lint("no-ex-assign");
    }

    #[test]
    fn test_allows_different_variable_assignment() {
        let test = TestProgram::for_rule_without_prelude(NoExAssign);
        let result = test.lint_ast(
            "test.ds",
            r#"
try {
    riskyOperation();
} catch e {
    let message = e.message;
    message = "modified";
}
"#,
        );
        test.result(result).assert_no_lint("no-ex-assign");
    }
}
