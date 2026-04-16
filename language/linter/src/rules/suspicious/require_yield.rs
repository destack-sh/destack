use crate::LintMeta;
use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_starts_nested_declaration_scope;
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Require a `yield` keyword in generator functions.
    ///
    /// Generator functions (`function*`) should contain at least one `yield`
    /// expression. A generator without `yield` is likely a mistake.
    #[lint(
        id = "require-yield",
        code = "LU044",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub RequireYield,
    "Require yield in generator functions"
}

impl LintRule for RequireYield {
    fn meta(&self) -> &'static LintMeta {
        RequireYield::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let decl = ctx.tree.get(node_id);

            let ast::Declaration::Function(declaration) = decl else {
                continue;
            };
            let Some(body_id) = declaration.body else {
                continue;
            };

            // check if this is a generator function
            if declaration.signature.cardinality != ast::FunctionCardinality::Generator {
                continue;
            }
            report_missing_generator_yield(
                ctx,
                meta,
                node_id,
                body_id,
                "generator function does not contain `yield`",
            );
        }

        // check generator methods
        for node_id in ctx.tree.iter_nodes::<ast::Member>() {
            let member = ctx.tree.get(node_id);

            let ast::Member::Method {
                signature,
                body: Some(body_id),
                ..
            } = member
            else {
                continue;
            };

            if signature.cardinality != ast::FunctionCardinality::Generator {
                continue;
            }
            report_missing_generator_yield(
                ctx,
                meta,
                node_id,
                *body_id,
                "generator method does not contain `yield`",
            );
        }

        // check generator object methods
        for node_id in ctx.tree.iter_nodes::<ast::Property>() {
            let property = ctx.tree.get(node_id);
            let ast::Property::Method {
                signature,
                body: Some(body_id),
                ..
            } = property
            else {
                continue;
            };

            if signature.cardinality != ast::FunctionCardinality::Generator {
                continue;
            }

            report_missing_generator_yield(
                ctx,
                meta,
                node_id,
                *body_id,
                "generator method does not contain `yield`",
            );
        }
    }
}

/// Report one generator callable that has no own yield expression.
fn report_missing_generator_yield<T: ast::Node>(
    ctx: &mut LintAstContext<'_>,
    meta: &LintMeta,
    callable_id: ast::LocalNodeId<T>,
    body_expression_id: ast::LocalNodeId<ast::Expression>,
    message: &str,
) {
    // allow empty generators
    if generator_body_is_empty(ctx.tree, body_expression_id) {
        return;
    }

    // skip callables that contain yield in their own body scope
    if generator_body_has_yield(ctx.tree, body_expression_id) {
        return;
    }

    // honor per node severity
    let callable_raw_id = callable_id.id;
    let severity = ctx.get_effective_severity(meta, ast::LocalNodeId::<T>::new(callable_raw_id));
    if !severity.is_enabled() {
        return;
    }

    let diagnostic = LintDiagnostic::new(
        REQUIRE_YIELD.id,
        REQUIRE_YIELD.code,
        REQUIRE_YIELD.category,
        severity,
        message,
        ctx.module.file_id,
        ctx.tree
            .get_span(ast::LocalNodeId::<T>::new(callable_raw_id)),
    )
    .with_label("add a `yield` expression or remove the `*`");

    ctx.report(diagnostic);
}

/// Return true when one generator body expression is an empty block.
fn generator_body_is_empty(
    tree: &ast::NodeTree,
    body_expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let body_expression = tree.get(body_expression_id);
    let ast::Expression::Block(block_id) = body_expression else {
        return false;
    };
    let block = tree.get(*block_id);

    block.is_empty()
}

/// Return true when one generator body has one yield expression in its own scope.
fn generator_body_has_yield(
    tree: &ast::NodeTree,
    body_expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let body_expression = tree.get(body_expression_id);
    let mut visitor = GeneratorYieldVisitor {
        options: NodeVisitorOptions::default(),
        has_yield: false,
    };
    visitor.visit_expression(tree, body_expression_id, body_expression);

    visitor.has_yield
}

/// Visitor that checks one generator body for yield expressions.
struct GeneratorYieldVisitor {
    /// Visitor options.
    options: NodeVisitorOptions,
    /// Whether a yield expression was seen in this callable scope.
    has_yield: bool,
}

impl NodeVisitor for GeneratorYieldVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // track direct yield expressions
        if matches!(expression, Expression::Yield { .. }) {
            self.has_yield = true;
            return;
        }

        // skip nested declaration scopes
        if expression_starts_nested_declaration_scope(expression) {
            return;
        }

        walk_expression(self, tree, expression_id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_generator_without_yield() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_detects_generator_without_yield.ds",
            r#"
function* gen() {
    return 1
}
"#,
        );
        test.result(result).assert_lint("require-yield");
    }

    #[test]
    fn test_reports_generator_without_fix() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_reports_generator_without_fix.ds",
            r#"
function* gen() {
    return 1
}
"#,
        );
        test.result(result)
            .assert_lint("require-yield")
            .assert_has_no_fix("require-yield");
    }

    #[test]
    fn test_allows_empty_generator() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_allows_empty_generator.ds",
            r#"
function* gen() {}
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }

    #[test]
    fn test_allows_generator_with_yield() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_allows_generator_with_yield.ds",
            r#"
function* gen() {
    yield 1
    yield 2
}
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }

    #[test]
    fn test_allows_generator_with_yield_in_loop() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_allows_generator_with_yield_in_loop.ds",
            r#"
function* gen() {
    for (const i of [0, 1, 2]) {
        yield i
    }
}
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }

    #[test]
    fn test_allows_regular_function() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_allows_regular_function.ds",
            r#"
function foo() {
    return 1
}
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }

    #[test]
    fn test_allows_generator_with_yield_star() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_allows_generator_with_yield_star.ds",
            r#"
function* gen() {
    yield* other()
}
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }

    #[test]
    fn test_reports_generator_method_without_fix() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_reports_generator_method_without_fix.ds",
            r#"
class C {
    *gen() {
        return 1
    }
}
"#,
        );
        test.result(result)
            .assert_lint("require-yield")
            .assert_has_no_fix("require-yield");
    }

    #[test]
    fn test_allows_empty_generator_method() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_allows_empty_generator_method.ds",
            r#"
class C {
    *gen() {}
}
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }

    #[test]
    fn test_flags_generator_function_expression_without_yield() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_flags_generator_function_expression_without_yield.ds",
            r#"
(function* gen() {
    return 1
})();
"#,
        );
        test.result(result).assert_lint("require-yield");
    }

    #[test]
    fn test_allows_empty_generator_function_expression() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_allows_empty_generator_function_expression.ds",
            r#"
(function* gen() {})();
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }

    #[test]
    fn test_flags_generator_object_method_without_yield() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_flags_generator_object_method_without_yield.ds",
            r#"
let obj = {
    *gen() {
        return 1
    }
};
"#,
        );
        test.result(result).assert_lint("require-yield");
    }

    #[test]
    fn test_flags_generator_with_only_nested_generator_yield() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_flags_generator_with_only_nested_generator_yield.ds",
            r#"
function* outer() {
    function* inner() {
        yield 1
    }
}
"#,
        );
        test.result(result)
            .assert_lint("require-yield")
            .assert_lint_count("require-yield", 1);
    }
}
