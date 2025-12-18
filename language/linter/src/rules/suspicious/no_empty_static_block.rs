use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty static initialization blocks in classes.
    ///
    /// Empty static blocks serve no purpose and are likely leftover from
    /// incomplete code. If intentional, add a comment explaining why.
    #[lint(
        id = "no-empty-static-block",
        code = "LU014",
        category = Suspicious,
        level = Ast
    )]
    pub NoEmptyStaticBlock,
    "Disallow empty static initialization blocks"
}

impl LintRule for NoEmptyStaticBlock {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEmptyStaticBlock::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Member>() {
            let member = ctx.tree.get(node_id);
            let ast::Member::StaticBlock { body, .. } = member else {
                continue;
            };

            let body_expr = ctx.tree.get(*body);
            // check if the body is a block expression with no statements
            let is_empty = match body_expr {
                ast::Expression::Block(block_id) => {
                    let block = ctx.tree.get(*block_id);
                    // check if block is empty and has no comments
                    block.expressions.is_empty() && !ctx.tree.has_infix_annotations(block_id.id)
                }
                _ => false,
            };

            if is_empty {
                ctx.report(
                    LintDiagnostic::new(
                        NO_EMPTY_STATIC_BLOCK.id,
                        NO_EMPTY_STATIC_BLOCK.code,
                        NO_EMPTY_STATIC_BLOCK.category,
                        severity,
                        "empty static initialization block",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("remove or add initialization code"),
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
    fn test_detects_empty_static_block() {
        let test = TestProgram::for_rule(NoEmptyStaticBlock);
        let result = test.lint_ast(
            "test.ts",
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
        let test = TestProgram::for_rule(NoEmptyStaticBlock);
        let result = test.lint_ast(
            "test.ts",
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
        let test = TestProgram::for_rule(NoEmptyStaticBlock);
        let result = test.lint_ast(
            "test.ts",
            r#"
class Foo {
    static { /* intentionally empty */ }
}
"#,
        );
        test.result(result).assert_no_lint("no-empty-static-block");
    }
}
