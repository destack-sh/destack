use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::block_is_empty_without_comment;
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleAstContext, LintRule, declare_lint};

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
    fn meta(&self) -> &'static LintMeta {
        NoEmptyFunction::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // inspect function declarations
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let ast::Declaration::Function { body, .. } = declaration else {
                continue;
            };
            let Some(body_expression_id) = body else {
                continue;
            };

            report_empty_function_body(ctx, meta, *body_expression_id);
        }

        // inspect class and object methods
        for member_id in ctx.tree.iter_nodes::<ast::Member>() {
            let member = ctx.tree.get(member_id);
            let ast::Member::Method { body, .. } = member else {
                continue;
            };
            let Some(body_expression_id) = body else {
                continue;
            };

            report_empty_function_body(ctx, meta, *body_expression_id);
        }
    }
}

/// Report one empty function-like body from declarations or methods.
fn report_empty_function_body(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static LintMeta,
    body_expression_id: ast::LocalNodeId<ast::Expression>,
) {
    // require an empty uncommented block body
    let Some(block_id) = empty_body_block_id(ctx, body_expression_id) else {
        return;
    };

    // skip disabled diagnostics
    let severity = ctx.get_effective_severity(meta, body_expression_id);
    if !severity.is_enabled() {
        return;
    }

    // build the empty function diagnostic
    let mut diagnostic = LintDiagnostic::new(
        NO_EMPTY_FUNCTION.id,
        NO_EMPTY_FUNCTION.code,
        NO_EMPTY_FUNCTION.category,
        severity,
        "empty function",
        ctx.module.file_id,
        ctx.tree.get_span(body_expression_id),
    )
    .with_label("add implementation or a comment explaining why empty");

    // attach a safe comment insertion fix when enabled
    if ctx.compute_fixes
        && let Some(fix) = no_empty_function_fix(ctx, block_id)
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Return one block id when a function body is empty and uncommented.
fn empty_body_block_id(
    ctx: &LintModuleAstContext<'_>,
    body_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Block>> {
    // require a block expression body
    let body_expression = ctx.tree.get(body_expression_id);
    let ast::Expression::Block(block_id) = body_expression else {
        return None;
    };

    // keep only empty uncommented blocks
    if block_is_empty_without_comment(ctx.tree, *block_id) {
        return Some(*block_id);
    }

    None
}

/// Build a safe fix that annotates an empty function block.
fn no_empty_function_fix(
    ctx: &LintModuleAstContext<'_>,
    block_id: ast::LocalNodeId<ast::Block>,
) -> Option<LintFix> {
    // replace the empty body with an explicit marker comment
    let block_span = ctx.tree.get_span(block_id);
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
    fn test_detects_empty_method() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint_ast(
            "no_empty_function/test_detects_empty_method.ds",
            r#"
class Foo {
    method() {}
}
"#,
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
