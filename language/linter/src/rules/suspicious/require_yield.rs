use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

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
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub RequireYield,
    "Require yield in generator functions"
}

impl LintRule for RequireYield {
    fn meta(&self) -> &'static crate::LintMeta {
        RequireYield::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

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
                let severity = ctx.get_effective_severity(meta, *body_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    REQUIRE_YIELD.id,
                    REQUIRE_YIELD.code,
                    REQUIRE_YIELD.category,
                    severity,
                    "generator function does not contain `yield`",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("add a `yield` expression or remove the `*`");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = remove_generator_marker_fix(ctx, node_id, *body_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
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
                let severity = ctx.get_effective_severity(meta, *body_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    REQUIRE_YIELD.id,
                    REQUIRE_YIELD.code,
                    REQUIRE_YIELD.category,
                    severity,
                    "generator method does not contain `yield`",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("add a `yield` expression or remove the `*`");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = remove_generator_marker_fix(ctx, node_id, *body_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build an unsafe fix by removing one generator marker (`*`) from a callable signature.
fn remove_generator_marker_fix<T: ast::Node>(
    ctx: &LintModuleAstContext<'_>,
    declaration_id: ast::LocalNodeId<T>,
    body_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let declaration_span = ctx.tree.get_span(declaration_id);
    let body_span = ctx.tree.get_span(body_expression_id);
    let header_span = Span::new(
        declaration_span.file,
        declaration_span.start,
        body_span.start,
    );
    let header_text = ctx.get_span_text(header_span);
    let star_offset = header_text.find('*')?;

    let marker_span = Span::new(
        header_span.file,
        header_span.start + star_offset as u32,
        header_span.start + star_offset as u32 + 1,
    );
    let edits = ctx.edit_builder().delete(marker_span).into_edits();
    Some(LintFix::r#unsafe("Remove generator marker").with_edits(edits))
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
    fn test_fix_removes_generator_marker_without_yield() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_fix_removes_generator_marker_without_yield.ds",
            r#"
function* gen() {
    return 1
}
"#,
        );
        test.result(result)
            .assert_lint("require-yield")
            .assert_unsafe_fixed(
                r#"
function gen() {
    return 1;
}
"#,
            );
    }

    #[test]
    fn test_detects_empty_generator() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_detects_empty_generator.ds",
            r#"
function* gen() {}
"#,
        );
        test.result(result).assert_lint("require-yield");
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
    fn test_mutation_fix_removes_generator_method_marker() {
        let test = TestProgram::for_rule_without_prelude(RequireYield);
        let result = test.lint_ast(
            "require_yield/test_mutation_fix_removes_generator_method_marker.ds",
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
            .assert_unsafe_fixed(
                r#"
class C {
    gen() {
        return 1;
    }
}
"#,
            );
    }
}
