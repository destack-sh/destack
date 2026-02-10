use destack_ast::{self as ast, ForEachKind, LocalNodeId};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

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
        code = "LU001",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub GuardForIn,
    "Require guard in for-in loops"
}

impl LintRule for GuardForIn {
    fn meta(&self) -> &'static crate::LintMeta {
        GuardForIn::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // only check for-in loops
            let ast::Expression::ForEach {
                kind: ForEachKind::In,
                binding,
                iterator,
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
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    GUARD_FOR_IN.id,
                    GUARD_FOR_IN.code,
                    GUARD_FOR_IN.category,
                    severity,
                    "for-in loop should have a hasOwnProperty guard",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("add if (Object.hasOwn(obj, key)) guard");
                if ctx.compute_fixes
                    && let Some(fix) = guard_for_in_fix(ctx, binding, *iterator, *body)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }
                ctx.report(diagnostic);
            }
        }
    }
}

/// Build an unsafe fix that wraps the body with Object.hasOwn guard.
fn guard_for_in_fix(
    ctx: &LintModuleAstContext<'_>,
    binding: &ast::ForEachBinding,
    iterator_expression_id: LocalNodeId<ast::Expression>,
    body_id: LocalNodeId<ast::Block>,
) -> Option<LintFix> {
    let key_name = for_in_binding_name(ctx, binding)?;
    let iterator_text = side_effect_free_iterator_text(ctx, iterator_expression_id)?;
    let body = ctx.tree.get(body_id);
    let inner_text = block_inner_text(ctx, body)?;
    let replacement =
        format!("{{ if (Object.hasOwn({iterator_text}, {key_name})) {{\n{inner_text}\n}} }}");

    let body_span = ctx.tree.get_span(body_id);
    let edits = ctx
        .edit_builder()
        .replace(body_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Wrap for-in body with Object.hasOwn guard").with_edits(edits))
}

/// Return the bound key name when the for-in binding is a simple identifier.
fn for_in_binding_name(
    ctx: &LintModuleAstContext<'_>,
    binding: &ast::ForEachBinding,
) -> Option<String> {
    let pattern_id = match binding {
        ast::ForEachBinding::Pattern { pattern, .. } => *pattern,
        ast::ForEachBinding::Using { pattern, .. } => *pattern,
    };
    let pattern = ctx.tree.get(pattern_id);
    let ast::Pattern::Binding {
        name,
        pattern: None,
        ..
    } = pattern
    else {
        return None;
    };

    Some(ctx.strings.get(*name).to_string())
}

/// Return iterator text only when it is side effect free.
fn side_effect_free_iterator_text(
    ctx: &LintModuleAstContext<'_>,
    expression_id: LocalNodeId<ast::Expression>,
) -> Option<String> {
    let expression = ctx.tree.get(expression_id);
    if !matches!(
        expression,
        ast::Expression::Path {
            static_arguments: None,
            ..
        }
    ) {
        return None;
    }

    Some(
        ctx.get_span_text(ctx.tree.get_span(expression_id))
            .to_string(),
    )
}

/// Return a block's inner source text for fix rewriting.
fn block_inner_text(ctx: &LintModuleAstContext<'_>, block: &ast::Block) -> Option<String> {
    let first_expression_id = *block.expressions.first()?;
    let last_expression_id = *block.expressions.last()?;
    let first_span = ctx.tree.get_span(first_expression_id);
    let last_span = ctx.tree.get_span(last_expression_id);
    let inner_span = destack_source::Span::new(first_span.file, first_span.start, last_span.end);
    Some(ctx.get_span_text(inner_span).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_unguarded_for_in_detected() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_ast(
            "guard_for_in/test_unguarded_for_in_detected.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {
        console.log(key)
    }
}
"#,
        );
        test.result(result)
            .assert_lint("guard-for-in")
            .assert_unsafe_fixed(
                r#"
function foo(obj: object) {
    for (const key in obj) {
        if (Object.hasOwn(obj, key)) {
            console.log(key)
        }
    }
}
"#,
            );
    }

    #[test]
    fn test_guarded_for_in_allowed() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_ast(
            "guard_for_in/test_guarded_for_in_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_ast(
            "guard_for_in/test_for_of_not_affected.ds",
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
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_ast(
            "guard_for_in/test_empty_for_in_allowed.ds",
            r#"
function foo(obj: object) {
    for (const key in obj) {}
}
"#,
        );
        test.result(result).assert_no_lint("guard-for-in");
    }

    #[test]
    fn test_no_fix_when_iterator_has_side_effects() {
        let test = TestProgram::for_rule_without_prelude(GuardForIn);
        let result = test.lint_ast(
            "guard_for_in/test_no_fix_when_iterator_has_side_effects.ds",
            r#"
function foo() {
    for (const key in getObject()) {
        console.log(key)
    }
}
"#,
        );
        test.result(result)
            .assert_lint("guard-for-in")
            .assert_has_no_fix("guard-for-in");
    }
}
