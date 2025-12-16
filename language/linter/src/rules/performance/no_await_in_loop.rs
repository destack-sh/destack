use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `await` inside loops.
    ///
    /// Using `await` in a loop causes sequential execution of async operations,
    /// which is often slower than running them in parallel with `Promise.all()`.
    ///
    /// Bad: `for (const url of urls) { await fetch(url); }`
    /// Good: `await Promise.all(urls.map(url => fetch(url)));`
    #[lint(
        id = "no-await-in-loop",
        code = "LP001",
        category = Performance,
        level = Ast
    )]
    pub NoAwaitInLoop,
    "Disallow await inside loops"
}

impl LintRule for NoAwaitInLoop {
    fn meta(&self) -> &'static crate::LintMeta {
        NoAwaitInLoop::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Await { .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // walk up the parent chain to check if we're inside a loop
            let mut current = node_id.id;
            while let Some(parent_id) = ctx.parents.get_by_id(current) {
                let parent_type = ctx.tree.get_node_type(parent_id);
                if parent_type != ast::NodeType::Expression {
                    current = parent_id;
                    continue;
                }

                let parent_expr_id = ast::LocalNodeId::<ast::Expression>::new(parent_id);
                let parent = ctx.tree.get(parent_expr_id);

                match parent {
                    // found a loop: report the await
                    ast::Expression::While { .. }
                    | ast::Expression::For { .. }
                    | ast::Expression::ForEach { .. }
                    | ast::Expression::Loop { .. } => {
                        ctx.report(
                            LintDiagnostic::new(
                                NO_AWAIT_IN_LOOP.id,
                                NO_AWAIT_IN_LOOP.code,
                                NO_AWAIT_IN_LOOP.category,
                                severity,
                                "`await` inside loop runs sequentially",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("consider using `Promise.all()` for parallel execution"),
                        );
                        break;
                    }
                    // found a function boundary: stop searching (await in nested async fn is fine)
                    ast::Expression::Declaration(decl_id) => {
                        let decl = ctx.tree.get(*decl_id);
                        if matches!(decl, ast::Declaration::Function { .. }) {
                            break;
                        }
                    }
                    _ => {}
                }

                current = parent_id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_await_in_for_loop() {
        let test = TestProgram::for_rule(NoAwaitInLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
async function fetchAll(urls: string[]) {
    for (const url of urls) {
        await fetch(url);
    }
}
"#,
        );
        test.result(result).assert_lint("no-await-in-loop");
    }

    #[test]
    fn test_detects_await_in_while_loop() {
        let test = TestProgram::for_rule(NoAwaitInLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
async function process() {
    while (hasMore()) {
        await processNext();
    }
}
"#,
        );
        test.result(result).assert_lint("no-await-in-loop");
    }

    #[test]
    fn test_detects_await_in_traditional_for() {
        let test = TestProgram::for_rule(NoAwaitInLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
async function fetchAll() {
    for (let i = 0; i < 10; i++) {
        await fetch(urls[i]);
    }
}
"#,
        );
        test.result(result).assert_lint("no-await-in-loop");
    }

    #[test]
    fn test_allows_await_outside_loop() {
        let test = TestProgram::for_rule(NoAwaitInLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
async function fetchOne(url: string) {
    const response = await fetch(url);
    return response.json();
}
"#,
        );
        test.result(result).assert_no_lint("no-await-in-loop");
    }

    #[test]
    fn test_allows_promise_all() {
        let test = TestProgram::for_rule(NoAwaitInLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
async function fetchAll(urls: string[]) {
    const results = await Promise.all(urls.map(url => fetch(url)));
    return results;
}
"#,
        );
        test.result(result).assert_no_lint("no-await-in-loop");
    }

    #[test]
    fn test_allows_await_in_nested_async_function() {
        // await inside a nested async function should not be flagged
        let test = TestProgram::for_rule(NoAwaitInLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
function process(items: int32[]) {
    for (const item of items) {
        const handler = async () => {
            await delay(100);
        };
        handler();
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-await-in-loop");
    }
}
