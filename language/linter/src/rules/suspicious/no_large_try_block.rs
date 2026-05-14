use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow large try blocks.
    ///
    /// Try blocks should be as small as possible, containing only the code
    /// that might throw. Large try blocks make it harder to understand
    /// which operation might fail and can accidentally catch errors from
    /// unrelated code.
    #[lint(
        id = "no-large-try-block",
        code = "LU021",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoLargeTryBlock,
    "Disallow large try blocks"
}

impl LintRule for NoLargeTryBlock {
    fn meta(&self) -> &'static LintMeta {
        NoLargeTryBlock::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let max_statements = ctx.options.complexity.max_try_block_statements;

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Try { try_expression, .. } = ctx.dir.get(node_id) else {
                continue;
            };

            // count statements in the try block
            let statement_count = count_statements(ctx, *try_expression);
            if statement_count > max_statements {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        NO_LARGE_TRY_BLOCK.id,
                        NO_LARGE_TRY_BLOCK.code,
                        NO_LARGE_TRY_BLOCK.category,
                        severity,
                        format!(
                            "try block has {statement_count} statements (max {max_statements})"
                        ),
                        ctx.dir.get_span(*try_expression),
                    )
                    .label("consider narrowing the try block to the specific failing code"),
                );
            }
        }
    }
}

/// Count statements in an expression.
fn count_statements(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> usize {
    let expression = ctx.dir.get(expression_id);
    match expression {
        dir::Expression::Block(block_id) => {
            let block = ctx.dir.get(*block_id);
            block.len()
        }
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_large_try_block() {
        let test = TestProgram::for_rule_without_prelude(NoLargeTryBlock)
            .with_options(|options| options.complexity.max_try_block_statements = 3);
        let result = test.lint(
            "no_large_try_block/test_detects_large_try_block.ds",
            r#"
try {
    doA();
    doB();
    doC();
    doD();
} catch (e) {
    handleError(e);
}
"#,
        );
        test.result(result).assert_lint("no-large-try-block");
    }

    #[test]
    fn test_allows_small_try_block() {
        let test = TestProgram::for_rule_without_prelude(NoLargeTryBlock)
            .with_options(|options| options.complexity.max_try_block_statements = 5);
        let result = test.lint(
            "no_large_try_block/test_allows_small_try_block.ds",
            r#"
try {
    doA();
    doB();
} catch (e) {
    handleError(e);
}
"#,
        );
        test.result(result).assert_no_lint("no-large-try-block");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_prelude(NoLargeTryBlock)
            .with_options(|options| options.complexity.max_try_block_statements = 3);
        let result = test.lint(
            "no_large_try_block/test_allows_exactly_at_limit.ds",
            r#"
try {
    doA();
    doB();
    doC();
} catch (e) {
    handleError(e);
}
"#,
        );
        test.result(result).assert_no_lint("no-large-try-block");
    }
}
