use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{is_array_type, is_async_function_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `forEach` with async callback.
    ///
    /// When an async function is passed to `forEach`, the returned promises
    /// are silently discarded. This is almost always a bug - use `for...of`
    /// with `await`, or `Promise.all()` with `map()` instead.
    #[lint(
        id = "no-async-foreach",
        code = "LU002",
        category = Suspicious,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoAsyncForeach,
    "Disallow forEach with async callback"
}

impl LintRule for NoAsyncForeach {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoAsyncForeach::meta()
    }

    /// Check module DIR nodes for async forEach callbacks.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = AsyncForeachVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags async forEach callbacks.
struct AsyncForeachVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The forEach method name.
    foreach_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> AsyncForeachVisitor<'a, 'b> {
    /// Build a visitor for async forEach checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let foreach_name = ctx.program.strings.intern("forEach");

        Self {
            ctx,
            meta,
            array_symbol,
            foreach_name,
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

    /// Check a call expression for async forEach callback.
    fn check_foreach_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // match member access for forEach method
        let left_expression = self.ctx.tree.get(left);
        let dir::Expression::Member {
            left: receiver,
            name,
            ..
        } = left_expression
        else {
            return;
        };

        // check method name
        if *name != self.foreach_name {
            return;
        }

        // ensure at least one argument (the callback)
        if arguments.is_empty() {
            return;
        }

        // resolve the receiver type
        let Some(receiver_type_id) = self.ctx.expression_type_id(*receiver) else {
            return;
        };

        // check if the receiver is an array type
        if !is_array_type(self.ctx.types, receiver_type_id, Some(self.array_symbol)) {
            return;
        }

        // get the callback argument
        let callback_arg_id = arguments[0];
        let callback_arg = self.ctx.tree.get(callback_arg_id);
        let dir::Argument::Positional { value, .. } = callback_arg else {
            return;
        };

        // check if the callback is an async function
        let Some(callback_type_id) = self.ctx.expression_type_id(*value) else {
            return;
        };

        if !is_async_function_type(self.ctx.types, callback_type_id) {
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
                NO_ASYNC_FOREACH.id,
                NO_ASYNC_FOREACH.code,
                NO_ASYNC_FOREACH.category,
                severity,
                "async callback in forEach will not be awaited",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use for...of with await, or Promise.all() with map()"),
        );
    }
}

impl NodeVisitor for AsyncForeachVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for async forEach
        if let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        {
            self.check_foreach_call(id, *left, dynamic_arguments);
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
    fn test_flags_async_foreach() {
        let test = TestProgram::for_rule_without_prelude(NoAsyncForeach);
        let result = test.lint_dir(
            "no_async_foreach/test_flags_async_foreach.ds",
            r#"
let items = [1, 2, 3];
items.forEach(async (item) => {
    await something(item);
});
"#,
        );
        test.result(result).assert_lint("no-async-foreach");
    }

    #[test]
    fn test_allows_sync_foreach() {
        let test = TestProgram::for_rule_without_prelude(NoAsyncForeach);
        let result = test.lint_dir(
            "no_async_foreach/test_allows_sync_foreach.ds",
            r#"
let items = [1, 2, 3];
items.forEach((item) => {
    console.log(item);
});
"#,
        );
        test.result(result).assert_no_lint("no-async-foreach");
    }

    #[test]
    fn test_allows_async_map() {
        let test = TestProgram::for_rule_without_prelude(NoAsyncForeach);
        let result = test.lint_dir(
            "no_async_foreach/test_allows_async_map.ds",
            r#"
let items = [1, 2, 3];
let results = items.map(async (item) => {
    return await something(item);
});
"#,
        );
        test.result(result).assert_no_lint("no-async-foreach");
    }
}
