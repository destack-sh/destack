use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{expression_target_symbol, is_async_function_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow async functions as Promise executors.
    ///
    /// Promise executors are expected to run synchronously and call resolve or reject.
    /// Async executors add an extra promise layer and can swallow errors.
    #[lint(
        id = "no-async-promise-executor",
        code = "LC007",
        category = Correctness,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Promise)],
        requires_any = [],
        fixable = No,
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
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = AsyncPromiseExecutorVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags async Promise executors.
struct AsyncPromiseExecutorVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Promise symbol for this module.
    promise_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> AsyncPromiseExecutorVisitor<'a, 'b> {
    /// Build a visitor for async Promise executor checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let promise_symbol = ctx.well_known_symbol(WellKnownSymbol::Promise);
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
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a Promise executor argument.
    fn check_promise_executor(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        dynamic_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // ignore non promise calls
        let Some(target_symbol) = expression_target_symbol(self.ctx.tree, left) else {
            return;
        };
        if target_symbol != self.promise_symbol {
            return;
        }

        // get the executor argument
        let Some(argument_id) = dynamic_arguments.first() else {
            return;
        };
        let argument = self.ctx.tree.get(*argument_id);
        let value_id = argument.value();

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
        self.ctx.report(
            LintDiagnostic::new(
                NO_ASYNC_PROMISE_EXECUTOR.id,
                NO_ASYNC_PROMISE_EXECUTOR.code,
                NO_ASYNC_PROMISE_EXECUTOR.category,
                severity,
                "async Promise executor detected",
                self.ctx.module.file_id,
                span,
            )
            .with_label("remove async from the executor function"),
        );
    }
}

impl NodeVisitor for AsyncPromiseExecutorVisitor<'_, '_> {
    /// Return visitor options.
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    /// Visit an expression node.
    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check Promise calls and constructors
        match expression {
            dir::Expression::Call {
                left,
                dynamic_arguments,
                ..
            } => self.check_promise_executor(id, *left, dynamic_arguments),
            dir::Expression::New {
                left,
                dynamic_arguments,
                ..
            } => self.check_promise_executor(id, *left, dynamic_arguments),
            _ => {}
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
            "test.ds",
            r#"
let task = new Promise(async (resolve, reject) => {
    resolve(1);
});
"#);
        test.result(result).assert_lint("no-async-promise-executor");
    }

    #[test]
    fn test_allows_sync_promise_executor() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "test.ds",
            r#"
let task = new Promise((resolve, reject) => {
    resolve(1);
});
"#);
        test.result(result)
            .assert_no_lint("no-async-promise-executor");
    }

    #[test]
    fn test_flags_async_executor_reference() {
        let test = TestProgram::for_rule_with_prelude(NoAsyncPromiseExecutor);
        let result = test.lint_dir(
            "test.ds",
            r#"
async function executor(resolve, reject) {
    resolve(1);
}

let task = new Promise(executor);
"#);
        test.result(result).assert_lint("no-async-promise-executor");
    }
}
