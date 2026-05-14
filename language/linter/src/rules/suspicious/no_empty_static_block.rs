use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::block_is_empty_without_comment;
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty static initialization blocks in classes.
    ///
    /// Empty static blocks serve no purpose and are likely leftover from
    /// incomplete code. If intentional, add a comment explaining why.
    #[lint(
        id = "no-empty-static-block",
        code = "LU015",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoEmptyStaticBlock,
    "Disallow empty static initialization blocks"
}

impl LintRule for NoEmptyStaticBlock {
    fn meta(&self) -> &'static LintMeta {
        NoEmptyStaticBlock::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect class members for static blocks
        for node_id in ctx.dir.iter_nodes::<dir::Member>() {
            let member = ctx.dir.get(node_id);
            let dir::Member::StaticBlock { body, .. } = member else {
                continue;
            };

            // require an empty uncommented block body
            let body_expr = ctx.dir.get(*body);
            let dir::Expression::Block(block_id) = body_expr else {
                continue;
            };
            if !block_is_empty_without_comment(ctx.dir.tree(), *block_id) {
                continue;
            }

            // skip disabled diagnostics
            let severity = ctx.get_effective_severity(meta, *body);
            if !severity.is_enabled() {
                continue;
            }

            // build the empty static block diagnostic
            let member_span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_EMPTY_STATIC_BLOCK.id,
                NO_EMPTY_STATIC_BLOCK.code,
                NO_EMPTY_STATIC_BLOCK.category,
                severity,
                "empty static initialization block",
                member_span,
            )
            .label("remove or add initialization code");

            // add comment insertion fix when enabled
            if ctx.compute_fixes {
                let block_span = ctx.dir.get_span(*block_id);
                let edits = ctx
                    .edit_builder()
                    .replace(block_span, "{\n        // intentionally empty\n    }")
                    .into_edits();
                let fix = LintFix::suggestion("Add intentional empty static block comment")
                    .with_edits(edits);
                diagnostic = diagnostic.fix(fix);
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
    fn test_detects_empty_static_block() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyStaticBlock);
        let result = test.lint(
            "no_empty_static_block/test_detects_empty_static_block.ts",
            r#"
class Foo {
    static {}
}
"#,
        );
        test.result(result).assert_lint("no-empty-static-block");
    }

    #[test]
    fn test_allows_static_block_with_code() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyStaticBlock);
        let result = test.lint(
            "no_empty_static_block/test_allows_static_block_with_code.ts",
            r#"
class Foo {
    static {
        console.log("initialized");
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-empty-static-block");
    }

    #[test]
    fn test_allows_static_block_with_comment() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyStaticBlock);
        let result = test.lint(
            "no_empty_static_block/test_allows_static_block_with_comment.ts",
            r#"
class Foo {
    static { /* intentionally empty */ }
}
"#,
        );
        test.result(result).assert_no_lint("no-empty-static-block");
    }

    #[test]
    fn test_fix_removes_empty_static_block() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyStaticBlock);
        let result = test.lint(
            "no_empty_static_block/test_fix_removes_empty_static_block.ts",
            r#"
class Foo {
    static {}
}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-static-block")
            .assert_suggested_fixed(
                r#"
class Foo {
    static {
        // intentionally empty
    }
}
"#,
            );
    }
}
