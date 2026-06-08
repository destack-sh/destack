use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

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
        level = Dir,
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata for per-node severity
        let meta = self.meta();

        // inspect await-like expressions that can serialize loop execution
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let Some(candidate) = await_loop_candidate(expression) else {
                continue;
            };

            // report `for (await using ... of ...)` directly
            if candidate == AwaitLoopCandidate::ForEachAwaitUsingBinding {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let (message, label) = await_loop_candidate_message(candidate);
                ctx.report(
                    LintReport::new(
                        NO_AWAIT_IN_LOOP.id,
                        NO_AWAIT_IN_LOOP.code,
                        NO_AWAIT_IN_LOOP.category,
                        severity,
                        message,
                        ctx.dir.get_span(node_id),
                    )
                    .label(label),
                );
                continue;
            }

            // walk up the parent chain to check if we're inside a loop
            let mut current = node_id.id;
            while let Some(parent_id) = ctx.dir.get_parent_id(current) {
                let parent_type = ctx.dir.get_node_type(parent_id);
                if parent_type != dir::NodeType::Expression {
                    current = parent_id;
                    continue;
                }

                let parent_expr_id = dir::LocalNodeId::<dir::Expression>::new(parent_id);
                let parent = ctx.dir.get(parent_expr_id);

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
                    let (message, label) = await_loop_candidate_message(candidate);
                    ctx.report(
                        LintReport::new(
                            NO_AWAIT_IN_LOOP.id,
                            NO_AWAIT_IN_LOOP.code,
                            NO_AWAIT_IN_LOOP.category,
                            severity,
                            message,
                            ctx.dir.get_span(node_id),
                        )
                        .label(label),
                    );
                    break;
                }

                current = parent_id;
            }
        }
    }
}

/// A kind of await-like candidate for no-await-in-loop reporting.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AwaitLoopCandidate {
    /// One `await` or `await?` expression.
    AwaitExpression,
    /// One `await using` declaration expression.
    AwaitUsingDeclaration,
    /// One `for (... await using ... of ...)` binding.
    ForEachAwaitUsingBinding,
}

/// Return one await-like candidate when this expression can serialize loop execution.
fn await_loop_candidate(expression: &dir::Expression) -> Option<AwaitLoopCandidate> {
    // match await expressions
    if matches!(
        expression,
        dir::Expression::Await { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. }
    ) {
        return Some(AwaitLoopCandidate::AwaitExpression);
    }

    // match await-using declarations
    if let dir::Expression::Using { asynchrony, .. } = expression
        && *asynchrony == dir::Asynchrony::Async
    {
        return Some(AwaitLoopCandidate::AwaitUsingDeclaration);
    }

    // match for-each bindings using await using
    if let dir::Expression::ForEach { binding, .. } = expression
        && let dir::ForEachBinding::Using { asynchrony, .. } = binding
        && *asynchrony == dir::Asynchrony::Async
    {
        return Some(AwaitLoopCandidate::ForEachAwaitUsingBinding);
    }

    None
}

/// Return diagnostic message and label text for one await-like candidate.
fn await_loop_candidate_message(candidate: AwaitLoopCandidate) -> (&'static str, &'static str) {
    match candidate {
        AwaitLoopCandidate::AwaitExpression => (
            "`await` inside loop runs sequentially",
            "consider using `Promise.all()` for parallel execution",
        ),
        AwaitLoopCandidate::AwaitUsingDeclaration => (
            "`await using` inside loop acquires resources sequentially",
            "consider acquiring resources outside the loop when possible",
        ),
        AwaitLoopCandidate::ForEachAwaitUsingBinding => (
            "`await using` in loop bindings runs per iteration",
            "consider restructuring resource acquisition to avoid per-iteration await",
        ),
    }
}

/// Return true when parent traversal should stop for this await expression.
fn is_boundary(parent: &dir::Expression, child_node_id: u32, ctx: &LintModuleContext<'_>) -> bool {
    // do not report awaits within `for await (...)` loops
    if let dir::Expression::ForEach { asynchrony, .. } = parent
        && *asynchrony == dir::Asynchrony::Async
    {
        return true;
    }

    // do not cross function declaration boundaries
    if let dir::Expression::Declaration(declaration_id) = parent {
        let declaration = ctx.dir.get(*declaration_id);
        if matches!(declaration, dir::Declaration::Function(_)) {
            return true;
        }
    }

    // sequence expression non-tail elements are not used per iteration
    if let dir::Expression::SequenceExpression { expressions } = parent
        && expressions
            .last()
            .is_some_and(|expression_id| expression_id.id != child_node_id)
    {
        return true;
    }

    false
}

/// Return true when this child position executes once per loop iteration.
fn is_looped_position(parent: &dir::Expression, child_node_id: u32) -> bool {
    match parent {
        dir::Expression::While {
            condition, body, ..
        } => condition.id == child_node_id || body.id == child_node_id,
        dir::Expression::For {
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
        dir::Expression::ForEach { body, .. } | dir::Expression::Loop { body } => {
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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

    #[test]
    fn test_detects_await_using_in_while_loop() {
        let test = TestProgram::for_rule_without_prelude(NoAwaitInLoop);
        let result = test.lint(
            "no_await_in_loop/test_detects_await_using_in_while_loop.ds",
            r#"
async function run(): Promise<void> {
    while (true) {
        await using resource = getResource();
        break;
    }
}
"#,
        );
        test.result(result).assert_lint("no-await-in-loop");
    }
}
