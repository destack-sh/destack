use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    block_is_empty_without_comment, block_is_function_body, block_is_static_block_body,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty block statements.
    ///
    /// Empty blocks are often a sign of incomplete code or accidental deletion.
    /// If intentional, add a comment explaining why the block is empty.
    #[lint(
        id = "no-empty",
        code = "LU012",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoEmpty,
    "Disallow empty block statements"
}

impl LintRule for NoEmpty {
    fn meta(&self) -> &'static LintMeta {
        NoEmpty::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // inspect explicit block nodes
        for node_id in ctx.tree.iter_nodes::<ast::Block>() {
            // skip implicit blocks, only explicit braces can be empty statements
            let block = ctx.tree.get(node_id);
            if block.format != ast::BlockFormat::Explicit {
                continue;
            }

            // keep only blocks without code and without comments
            if !block_is_empty_without_comment(ctx.tree, node_id) {
                continue;
            }

            // allow empty function and static block bodies
            if block_is_function_body(ctx.tree, ctx.parents, node_id)
                || block_is_static_block_body(ctx.tree, ctx.parents, node_id)
            {
                continue;
            }

            // skip disabled diagnostics
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build the diagnostic for this empty block
            let span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintDiagnostic::new(
                NO_EMPTY.id,
                NO_EMPTY.code,
                NO_EMPTY.category,
                severity,
                "empty block statement",
                ctx.module.file_id,
                span,
            )
            .with_label("this block is empty");

            // add an intent preserving comment fix when requested
            if ctx.compute_fixes {
                let edits = ctx
                    .edit_builder()
                    .replace(span, "{\n    // intentionally empty\n}")
                    .into_edits();
                let fix = LintFix::safe("Add intentional empty block comment").with_edits(edits);
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_block() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint_ast(
            "no_empty/test_detects_empty_block.ds",
            r#"
{}
"#,
        );
        test.result(result)
            .assert_lint("no-empty")
            .assert_has_fix("no-empty");
    }

    #[test]
    fn test_detects_empty_if_block() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint_ast(
            "no_empty/test_detects_empty_if_block.ds",
            r#"
if (true) {}
"#,
        );
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_allows_empty_function_body() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint_ast(
            "no_empty/test_allows_empty_function_body.ds",
            r#"
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_no_empty_with_content() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint_ast(
            "no_empty/test_no_empty_with_content.ds",
            r#"
{ let x = 1; }
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_no_empty_module_level() {
        // implicit module level blocks should not trigger
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint_ast(
            "no_empty/test_no_empty_module_level.ds",
            r#"
let x = 1;
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_no_empty_block_with_comment() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint_ast(
            "no_empty/test_no_empty_block_with_comment.ds",
            r#"
{ /* intentionally empty */ }
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_fix_adds_comment_to_empty_block() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint_ast(
            "no_empty/test_fix_adds_comment_to_empty_block.ds",
            r#"
{}
"#,
        );
        test.result(result)
            .assert_lint("no-empty")
            .assert_safe_fixed(
                r#"
{
    // intentionally empty
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_adds_comment_to_empty_if_block() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint_ast(
            "no_empty/test_mutation_fix_adds_comment_to_empty_if_block.ds",
            r#"
if (ready) {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty")
            .assert_safe_fixed(
                r#"
if (ready) {
    // intentionally empty
}
"#,
            );
    }
}
