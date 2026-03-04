use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintMeta, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `await` inside loops.
    ///
    /// Using `await` in a loop causes sequential execution of async operations,
    /// which is often slower than running them in parallel with `Promise.all()`.
    ///
    /// bad: `for (const url of urls) { await fetch(url); }`
    /// good: `await Promise.all(urls.map(url => fetch(url)));`
    #[lint(
        id = "no-await-in-loop",
        code = "LP004",
        category = Performance,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoAwaitInLoop,
    "Disallow await inside loops"
}

impl LintRule for NoAwaitInLoop {
    fn meta(&self) -> &'static LintMeta {
        NoAwaitInLoop::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(
                expression,
                ast::Expression::Await { .. } | ast::Expression::AwaitMaybe { .. }
            ) {
                continue;
            }

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

                // stop traversal at async loop boundaries and function declarations
                if is_boundary(parent, current, ctx) {
                    break;
                }

                // report await only for per-iteration loop positions
                if is_looped_position(parent, current) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        break;
                    }
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

                current = parent_id;
            }
        }
    }
}

/// Return true when parent traversal should stop for this await expression.
fn is_boundary(
    parent: &ast::Expression,
    child_node_id: u32,
    ctx: &LintModuleAstContext<'_>,
) -> bool {
    // do not report awaits within `for await (...)` loops
    if let ast::Expression::ForEach { asynchrony, .. } = parent
        && *asynchrony == ast::Asynchrony::Async
    {
        return true;
    }

    // do not cross function declaration boundaries
    if let ast::Expression::Declaration(declaration_id) = parent {
        let declaration = ctx.tree.get(*declaration_id);
        if matches!(declaration, ast::Declaration::Function { .. }) {
            return true;
        }
    }

    // sequence expression non-tail elements are not used per iteration
    if let ast::Expression::SequenceExpression { expressions } = parent
        && expressions
            .last()
            .is_some_and(|expression_id| expression_id.id != child_node_id)
    {
        return true;
    }

    false
}

/// Return true when this child position executes once per loop iteration.
fn is_looped_position(parent: &ast::Expression, child_node_id: u32) -> bool {
    match parent {
        ast::Expression::While {
            condition, body, ..
        } => condition.id == child_node_id || body.id == child_node_id,
        ast::Expression::For {
            condition,
            increment,
            body,
            ..
        } => {
            condition
                .as_ref()
                .is_some_and(|condition_id| condition_id.id == child_node_id)
                || increment
                    .as_ref()
                    .is_some_and(|increment_id| increment_id.id == child_node_id)
                || body.id == child_node_id
        }
        ast::Expression::ForEach { body, .. } | ast::Expression::Loop { body } => {
            body.id == child_node_id
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_await_in_for_loop() {
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint_ast(
            "no_await_in_loop/test_detects_await_in_for_loop.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint_ast(
            "no_await_in_loop/test_detects_await_in_while_loop.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint_ast(
            "no_await_in_loop/test_detects_await_in_traditional_for.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint_ast(
            "no_await_in_loop/test_allows_await_outside_loop.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint_ast(
            "no_await_in_loop/test_allows_promise_all.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint_ast(
            "no_await_in_loop/test_allows_await_in_nested_async_function.ds",
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

    #[test]
    fn test_allows_await_in_for_initialization() {
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint_ast(
            "no_await_in_loop/test_allows_await_in_for_initialization.ds",
            r#"
async function seed(): Promise<int32> {
    return 0;
}

async function run(): Promise<void> {
    for (let i = await seed(); i < 3; i += 1) {
        console.log(i);
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-await-in-loop");
    }

    #[test]
    fn test_allows_await_in_for_each_iterator() {
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint_ast(
            "no_await_in_loop/test_allows_await_in_for_each_iterator.ds",
            r#"
async function values(): Promise<int32[]> {
    return [1, 2, 3];
}

async function run(): Promise<void> {
    for (const value of await values()) {
        console.log(value);
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-await-in-loop");
    }
}
