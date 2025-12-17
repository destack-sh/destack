use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require a `yield` keyword in generator functions.
    ///
    /// Generator functions (`function*`) should contain at least one `yield`
    /// expression. A generator without `yield` is likely a mistake.
    #[lint(
        id = "require-yield",
        code = "LU021",
        category = Suspicious,
        level = Ast
    )]
    pub RequireYield,
    "Require yield in generator functions"
}

impl LintRule for RequireYield {
    fn meta(&self) -> &'static crate::LintMeta {
        RequireYield::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // collect all yield expression spans
        let mut yield_spans: Vec<destack_source::Span> = Vec::new();
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            if matches!(expr, ast::Expression::Yield { .. }) {
                yield_spans.push(ctx.tree.get_span(node_id));
            }
        }

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let decl = ctx.tree.get(node_id);

            let ast::Declaration::Function {
                signature,
                body: Some(body_id),
                ..
            } = decl
            else {
                continue;
            };

            // check if this is a generator function
            if signature.cardinality != ast::FunctionCardinality::Generator {
                continue;
            }

            // check if any yield is within the function body
            let body_span = ctx.tree.get_span(*body_id);
            let has_yield = yield_spans
                .iter()
                .any(|s| s.start >= body_span.start && s.end <= body_span.end);

            if !has_yield {
                ctx.report(
                    LintDiagnostic::new(
                        REQUIRE_YIELD.id,
                        REQUIRE_YIELD.code,
                        REQUIRE_YIELD.category,
                        severity,
                        "generator function does not contain `yield`",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add a `yield` expression or remove the `*`"),
                );
            }
        }

        // also check generator methods
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

            // check if any yield is within the method body
            let body_span = ctx.tree.get_span(*body_id);
            let has_yield = yield_spans
                .iter()
                .any(|s| s.start >= body_span.start && s.end <= body_span.end);

            if !has_yield {
                ctx.report(
                    LintDiagnostic::new(
                        REQUIRE_YIELD.id,
                        REQUIRE_YIELD.code,
                        REQUIRE_YIELD.category,
                        severity,
                        "generator method does not contain `yield`",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add a `yield` expression or remove the `*`"),
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
    fn test_detects_generator_without_yield() {
        let test = TestProgram::for_rule(RequireYield);
        let result = test.lint_ast(
            "test.ds",
            r#"
function* gen() {
    return 1
}
"#,
        );
        test.result(result).assert_lint("require-yield");
    }

    #[test]
    fn test_detects_empty_generator() {
        let test = TestProgram::for_rule(RequireYield);
        let result = test.lint_ast(
            "test.ds",
            r#"
function* gen() {}
"#,
        );
        test.result(result).assert_lint("require-yield");
    }

    #[test]
    fn test_allows_generator_with_yield() {
        let test = TestProgram::for_rule(RequireYield);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(RequireYield);
        let result = test.lint_ast(
            "test.ds",
            r#"
function* gen() {
    for (const i of 0..10) {
        yield i
    }
}
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }

    #[test]
    fn test_allows_regular_function() {
        let test = TestProgram::for_rule(RequireYield);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(RequireYield);
        let result = test.lint_ast(
            "test.ds",
            r#"
function* gen() {
    yield* other()
}
"#,
        );
        test.result(result).assert_no_lint("require-yield");
    }
}
