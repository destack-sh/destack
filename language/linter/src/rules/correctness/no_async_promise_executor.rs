use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{
    CallLikeExpressionInfo, expression_call_like, expression_target_symbol,
    expression_unwrap_parenthesized, is_async_function_type, remove_first_async_keyword,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow async functions as Promise executors.
    ///
    /// Promise executors are expected to run synchronously and call resolve or reject.
    /// Async executors add an extra promise layer and can swallow errors.
    #[lint(
        id = "no-async-promise-executor",
        code = "LC005",
        category = Correctness,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Promise)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoAsyncPromiseExecutor,
    "Disallow async Promise executor functions"
}

impl LintRule for NoAsyncPromiseExecutor {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoAsyncPromiseExecutor::meta()
    }

    /// Check module DIR nodes for async Promise executors.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk module expressions
        let mut visitor = AsyncPromiseExecutorVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags async Promise executors.
struct AsyncPromiseExecutorVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item Promise symbol for this module.
    promise_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> AsyncPromiseExecutorVisitor<'a, 'b> {
    /// Build a visitor for async Promise executor checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let promise_symbol = ctx.language_item(LanguageItem::Promise);
        Self {
            ctx,
            meta,
            promise_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a Promise executor argument.
    fn check_promise_executor(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        call_like: CallLikeExpressionInfo<'_>,
    ) {
        // ignore non promise calls
        let Some(target_symbol) = expression_target_symbol(self.ctx, call_like.left) else {
            return;
        };
        if target_symbol != self.promise_symbol {
            return;
        }

        // get the executor argument
        let Some(argument_id) = call_like.arguments.first() else {
            return;
        };
        let argument = self.ctx.dir.get(*argument_id);
        let Some(value_id) = argument.value() else {
            return;
        };

        // resolve the executor type
        let Some(type_id) = self.ctx.expression_type_id(value_id) else {
            return;
        };
        if !is_async_function_type(self.ctx.types, type_id) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            NO_ASYNC_PROMISE_EXECUTOR.id,
            NO_ASYNC_PROMISE_EXECUTOR.code,
            NO_ASYNC_PROMISE_EXECUTOR.category,
            severity,
            "async Promise executor detected",
            span,
        )
        .label("remove async from the executor function");

        // compute fixes only when requested by the runner
        if self.ctx.compute_fixes
            && let Some(fix) = async_promise_executor_fix(self.ctx, value_id)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

/// Build an unsafe fix that removes the async modifier from an inline executor.
fn async_promise_executor_fix(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    // require a direct inline declaration expression
    let expression_id = expression_unwrap_parenthesized(ctx.dir.tree(), expression_id);
    let expression = ctx.dir.get(expression_id);
    if !matches!(expression, dir::Expression::Declaration { .. }) {
        return None;
    }

    // strip one leading async keyword
    let expression_span = ctx.get_span(expression_id);
    let expression_text = ctx.get_span_text(expression_span);
    let rewritten = remove_first_async_keyword(expression_text)?;
    if rewritten == expression_text {
        return None;
    }

    // remove one redundant parenthesized wrapper when the rewritten executor
    // already has the arrow function's own parameter parentheses
    let replacement_span = if let Some(parent) = ctx.dir.get_parent(expression_id.id)
        && parent.ty == dir::NodeType::Expression
    {
        let parent_id = parent.into_typed::<dir::Expression>();
        let parent_expression = ctx.dir.get(parent_id);
        if matches!(
            parent_expression,
            dir::Expression::Parenthesized {
                expression
            } if *expression == expression_id
        ) {
            ctx.get_span(parent_id)
        } else {
            expression_span
        }
    } else {
        expression_span
    };

    // build replacement edit
    let edits = ctx
        .edit_builder()
        .replace(replacement_span, rewritten)
        .into_edits();

    // return unsafe rewrite fix
    Some(LintFix::r#unsafe("Remove async from Promise executor").with_edits(edits))
}

impl NodeVisitor for AsyncPromiseExecutorVisitor<'_, '_> {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit an expression node.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check Promise calls and constructors
        if let Some(call_like) = expression_call_like(expression) {
            self.check_promise_executor(id, call_like);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_async_promise_executor() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "no_async_promise_executor/test_flags_async_promise_executor.ds",
            r#"
let task = new Promise(async (resolve, reject) => {
    resolve(1);
});
"#,
        );
        test.result(result)
            .assert_lint("no-async-promise-executor")
            .assert_unsafe_fixed(
                r#"
let task = new Promise((resolve, reject) => {
    resolve(1);
});
"#,
            );
    }

    #[test]
    fn test_allows_sync_promise_executor() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "no_async_promise_executor/test_allows_sync_promise_executor.ds",
            r#"
let task = new Promise((resolve, reject) => {
    resolve(1);
});
"#,
        );
        test.result(result)
            .assert_no_lint("no-async-promise-executor");
    }

    #[test]
    fn test_allows_async_executor_for_non_promise_constructor() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "no_async_promise_executor/test_allows_async_executor_for_non_promise_constructor.ds",
            r#"
function Foo(executor) {}

let task = new Foo(async (resolve, reject) => {
    resolve(1);
});
"#,
        );
        test.result(result)
            .assert_no_lint("no-async-promise-executor");
    }

    #[test]
    fn test_flags_async_executor_reference() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "no_async_promise_executor/test_flags_async_executor_reference.ds",
            r#"
async function executor(resolve, reject) {
    resolve(1);
}

let task = new Promise(executor);
"#,
        );
        test.result(result)
            .assert_lint("no-async-promise-executor")
            .assert_has_no_fix("no-async-promise-executor");
    }

    #[test]
    fn test_fix_removes_async_from_function_executor_expression() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "no_async_promise_executor/test_fix_removes_async_from_function_executor_expression.ds",
            r#"
let task = new Promise(async function(resolve, reject) {
    resolve(1);
});
"#,
        );
        test.result(result)
            .assert_lint("no-async-promise-executor")
            .assert_unsafe_fixed(
                r#"
let task = new Promise(function (resolve, reject) {
    resolve(1);
});
"#,
            );
    }

    #[test]
    fn test_mutation_fix_removes_async_from_parenthesized_arrow_executor() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "no_async_promise_executor/test_mutation_fix_removes_async_from_parenthesized_arrow_executor.ds",
            r#"
let task = new Promise((async (resolve, reject) => {
    resolve(1);
}));
"#,
        );
        test.result(result)
            .assert_lint("no-async-promise-executor")
            .assert_unsafe_fixed(
                r#"
let task = new Promise((resolve, reject) => {
    resolve(1);
});
"#,
            );
    }

    #[test]
    fn test_flags_async_promise_executor_call_form() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "no_async_promise_executor/test_flags_async_promise_executor_call_form.ds",
            r#"
let task = Promise(async (resolve, reject) => {
    resolve(1);
});
"#,
        );
        test.result(result)
            .assert_lint("no-async-promise-executor")
            .assert_unsafe_fixed(
                r#"
let task = Promise((resolve, reject) => {
    resolve(1);
});
"#,
            );
    }
}
