use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::span_has_comment_trivia;
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty functions.
    ///
    /// Empty functions are often a sign of incomplete code. If intentional,
    /// add a comment explaining why the function is empty.
    #[lint(
        id = "no-empty-function",
        code = "LU013",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoEmptyFunction,
    "Disallow empty functions"
}

impl LintRule for NoEmptyFunction {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEmptyFunction::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let ast::Declaration::Function {
                body: Some(body_id),
                ..
            } = declaration
            else {
                continue;
            };

            // check if body is an empty block
            let body = ctx.tree.get(*body_id);
            let is_empty = match body {
                ast::Expression::Block(block_id) => {
                    let block = ctx.tree.get(*block_id);
                    let block_span = ctx.tree.get_span(*block_id);
                    let block_has_comment = span_has_comment_trivia(ctx.tree, block_span);
                    block.expressions.is_empty() && !block_has_comment
                }
                _ => false,
            };

            if is_empty {
                let severity = ctx.get_effective_severity(meta, *body_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    NO_EMPTY_FUNCTION.id,
                    NO_EMPTY_FUNCTION.code,
                    NO_EMPTY_FUNCTION.category,
                    severity,
                    "empty function",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("add implementation or a comment explaining why empty");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = no_empty_function_fix(ctx, *body_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build a safe fix that annotates an empty function block.
fn no_empty_function_fix(
    ctx: &LintModuleAstContext<'_>,
    body_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let body_expression = ctx.tree.get(body_id);
    let ast::Expression::Block(block_id) = body_expression else {
        return None;
    };

    let block_span = ctx.tree.get_span(*block_id);
    let edits = ctx
        .edit_builder()
        .replace(block_span, "{\n    // intentionally empty\n}")
        .into_edits();
    Some(LintFix::safe("Add intentional empty function comment").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_function() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint_ast(
            "no_empty_function/test_detects_empty_function.ds",
            "function foo() {}",
        );
        test.result(result)
            .assert_lint("no-empty-function")
            .assert_has_fix("no-empty-function");
    }

    #[test]
    fn test_detects_empty_arrow_function() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint_ast(
            "no_empty_function/test_detects_empty_arrow_function.ds",
            "const foo = () => {}",
        );
        test.result(result).assert_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_with_body() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint_ast(
            "no_empty_function/test_allows_function_with_body.ds",
            "function foo() { return 1; }",
        );
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_with_comment() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint_ast(
            "no_empty_function/test_allows_function_with_comment.ds",
            "function foo() { /* intentionally empty */ }",
        );
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_declaration_without_body() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint_ast(
            "no_empty_function/test_allows_function_declaration_without_body.ts",
            "declare function foo(): void;",
        );
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_fix_adds_comment_to_empty_function() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint_ast(
            "no_empty_function/test_fix_adds_comment_to_empty_function.ds",
            r#"
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-function")
            .assert_safe_fixed(
                r#"
function foo() {
    // intentionally empty
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_adds_comment_to_empty_arrow_function() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint_ast(
            "no_empty_function/test_mutation_fix_adds_comment_to_empty_arrow_function.ds",
            r#"
const foo = () => {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-function")
            .assert_safe_fixed(
                r#"
const foo = () => {
    // intentionally empty
};
"#,
            );
    }
}
