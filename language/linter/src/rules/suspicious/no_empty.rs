use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    block_expression_ancestor, block_is_empty_without_comment, block_is_function_body,
    block_is_static_block_body, span_has_comment,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty block statements.
    ///
    /// Empty blocks are often a sign of incomplete code or accidental deletion.
    /// If intentional, add a comment explaining why the block is empty.
    #[lint(
        id = "no-empty",
        code = "LU012",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect explicit block nodes
        for node_id in ctx.dir.iter_nodes::<dir::Block>() {
            // skip implicit blocks, only explicit braces can be empty statements
            let block = ctx.dir.get(node_id);
            if !block.is_explicit() {
                continue;
            }

            // keep only blocks without code and without comments
            if !block_is_empty_without_comment(ctx.dir.tree(), node_id) {
                continue;
            }

            // allow empty function and static block bodies
            if block_is_function_body(ctx.dir.tree(), node_id)
                || block_is_static_block_body(ctx.dir.tree(), node_id)
            {
                continue;
            }
            if ctx.options().correctness.no_empty_allow_empty_catch
                && block_is_catch_body(ctx, node_id)
            {
                continue;
            }

            // skip disabled diagnostics
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // build the diagnostic for this empty block
            let span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_EMPTY.id,
                NO_EMPTY.code,
                NO_EMPTY.category,
                severity,
                "empty block statement",
                span,
            )
            .label("this block is empty");

            // add an intent preserving comment fix when requested
            if ctx.compute_fixes {
                let edits = ctx
                    .edit_builder()
                    .replace(span, "{\n    // intentionally empty\n}")
                    .into_edits();
                let fix = LintFix::safe("Add intentional empty block comment").with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }

        // inspect empty switch expressions separately
        for expression_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Match { form, cases, .. } = ctx.dir.get(expression_id) else {
                continue;
            };
            if *form != dir::MatchForm::Switch || !cases.is_empty() {
                continue;
            }
            if span_has_comment(ctx.dir.tree(), ctx.dir.get_span(expression_id)) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintReport::new(
                    NO_EMPTY.id,
                    NO_EMPTY.code,
                    NO_EMPTY.category,
                    severity,
                    "empty switch statement",
                    ctx.dir.get_span(expression_id),
                )
                .label("this switch has no cases"),
            );
        }
    }
}

/// Return true when one explicit block is the catch body of a try expression.
fn block_is_catch_body(
    ctx: &LintModuleContext<'_>,
    block_id: dir::LocalNodeId<dir::Block>,
) -> bool {
    // resolve the owning block expression first
    let Some(block_expression_id) = block_expression_ancestor(ctx.dir.tree(), block_id) else {
        return false;
    };

    // keep only try catch bodies
    let Some(parent_id) = ctx.dir.get_parent_id(block_expression_id.id) else {
        return false;
    };
    if ctx.dir.get_node_type(parent_id) != dir::NodeType::Catch {
        return false;
    }

    let catch_id = dir::LocalNodeId::<dir::Catch>::new(parent_id);
    let catch = ctx.dir.get(catch_id);

    catch.body == block_expression_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_block() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "no_empty/test_no_empty_block_with_comment.ds",
            r#"
{ /* intentionally empty */ }
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_detects_empty_switch() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint(
            "no_empty/test_detects_empty_switch.ds",
            r#"
switch (value) {}
"#,
        );
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_allows_empty_switch_with_comment() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint(
            "no_empty/test_allows_empty_switch_with_comment.ds",
            r#"
switch (value) { /* intentionally empty */ }
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_allows_empty_catch_when_configured() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty).with_options(|options| {
            options.correctness.no_empty_allow_empty_catch = true;
        });
        let result = test.lint(
            "no_empty/test_allows_empty_catch_when_configured.ds",
            r#"
try {
    work()
} catch (error) {}
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_fix_adds_comment_to_empty_block() {
        let test = TestProgram::for_rule_without_prelude(NoEmpty);
        let result = test.lint(
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
        let result = test.lint(
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
