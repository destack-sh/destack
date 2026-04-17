use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    callable_return_usage, expression_target_symbol, expression_unwrap_transparent,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow returning values from Promise executors.
    ///
    /// Promise executors should call resolve or reject instead of returning values.
    #[lint(
        id = "no-promise-executor-return",
        code = "LC024",
        category = Correctness,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Promise)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoPromiseExecutorReturn,
    "Disallow returning values from Promise executors"
}

impl LintRule for NoPromiseExecutorReturn {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoPromiseExecutorReturn::meta()
    }

    /// Check module DIR nodes for Promise executor returns.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PromiseExecutorReturnVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags Promise executor returns.
struct PromiseExecutorReturnVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Promise symbol for this module.
    promise_symbol: dir::GlobalSymbolId,
    /// Allow explicit `void` returns from Promise executors.
    allow_void: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PromiseExecutorReturnVisitor<'a, 'b> {
    /// Build a visitor for Promise executor return checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let promise_symbol = ctx.well_known_symbol(WellKnownSymbol::Promise);
        let allow_void = ctx
            .options
            .correctness
            .no_promise_executor_return_allow_void;
        Self {
            ctx,
            meta,
            promise_symbol,
            allow_void,
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

    /// Check a Promise executor argument for return values.
    fn check_executor_returns(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // ignore non promise calls
        let Some(target_symbol) = expression_target_symbol(self.ctx.tree, left) else {
            return;
        };
        if target_symbol != self.promise_symbol {
            return;
        }

        // get the executor argument
        let Some(argument_id) = arguments.first() else {
            return;
        };
        let argument = self.ctx.tree.get(*argument_id);
        let value_id = argument.value();

        // resolve the executor declaration
        let Some(declaration_id) = executor_declaration(self.ctx, value_id) else {
            return;
        };
        let analysis = analyze_executor_returns(self.ctx.tree, declaration_id, self.allow_void);
        if !analysis.returns_value() {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            NO_PROMISE_EXECUTOR_RETURN.id,
            NO_PROMISE_EXECUTOR_RETURN.code,
            NO_PROMISE_EXECUTOR_RETURN.category,
            severity,
            "avoid returning values from Promise executors",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use resolve or reject instead of returning");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && !analysis.has_expression_body_return_value
            && let Some(fix) = promise_executor_return_fix(self.ctx, &analysis.return_value_nodes)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

impl NodeVisitor for PromiseExecutorReturnVisitor<'_, '_> {
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
        // check Promise constructors only
        if let dir::Expression::New {
            left, arguments, ..
        } = expression
        {
            self.check_executor_returns(id, *left, arguments);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Resolve a Promise executor declaration from an expression.
fn executor_declaration(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Declaration>> {
    // handle inline function declarations
    let expression = ctx.tree.get(expression_id);
    if let dir::Expression::Declaration(declaration) = expression {
        return Some(*declaration);
    }

    // resolve referenced declarations
    let target_symbol = expression_target_symbol(ctx.tree, expression_id)?;
    if target_symbol.module_id != ctx.module.id {
        return None;
    }

    let symbol_entry = ctx.symbols.get_symbol(target_symbol.local_id);
    let primary_declaration = symbol_entry.primary_declaration?;
    if primary_declaration.module_id != ctx.module.id {
        return None;
    }

    if primary_declaration.local_id.ty != dir::NodeType::Declaration {
        return None;
    }

    Some(primary_declaration.into_local_typed())
}

/// Check whether a function declaration returns a value.
fn analyze_executor_returns(
    tree: &dir::NodeTree,
    declaration_id: dir::LocalNodeId<dir::Declaration>,
    allow_void: bool,
) -> crate::rules::common::CallableReturnUsage {
    // extract the function body
    let declaration = tree.get(declaration_id);
    let dir::Declaration::Function(declaration) = declaration else {
        return crate::rules::common::CallableReturnUsage::default();
    };

    let mut usage = callable_return_usage(tree, &declaration.signature, declaration.body);

    // allow concise `() => void expr` bodies when configured
    if usage.has_expression_body_return_value
        && allow_void
        && declaration
            .body
            .is_some_and(|body_id| expression_is_void_operator(tree, body_id))
    {
        usage.has_expression_body_return_value = false;
    }

    // allow `return void expr` when configured
    if allow_void {
        usage.return_value_nodes.retain(|return_id| {
            let dir::Expression::Return {
                value: Some(value_id),
            } = tree.get(*return_id)
            else {
                return false;
            };

            !expression_is_void_operator(tree, *value_id)
        });
        usage.has_return_value = !usage.return_value_nodes.is_empty();
    }

    usage
}

/// Return true when one expression is a `void` unary expression.
fn expression_is_void_operator(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    matches!(
        tree.get(expression_id),
        dir::Expression::Unary {
            operator: dir::UnaryOperator::Void,
            ..
        }
    )
}

/// Build an unsafe fix for explicit Promise executor return values.
fn promise_executor_return_fix(
    ctx: &LintModuleDirContext<'_>,
    return_nodes: &[dir::LocalNodeId<dir::Expression>],
) -> Option<LintFix> {
    let mut builder = ctx.edit_builder();
    let mut replaced_count = 0_usize;

    for return_id in return_nodes {
        let return_expression = ctx.tree.get(*return_id);
        let dir::Expression::Return {
            value: Some(value_id),
        } = return_expression
        else {
            continue;
        };

        let value_span = ctx.get_span(*value_id);
        let value_text = ctx.get_span_text(value_span);
        if value_text.trim().is_empty() {
            return None;
        }

        let replacement = format!("{{ {value_text}; return; }}");
        let return_span = ctx.get_span(*return_id);
        builder = builder.replace(return_span, replacement);
        replaced_count += 1;
    }

    if replaced_count == 0 {
        return None;
    }

    let edits = builder.into_edits();
    Some(LintFix::r#unsafe("Drop Promise executor return value").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_executor_return_value() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_flags_executor_return_value.ds",
            r#"
let task = new Promise((resolve, reject) => {
    return 1;
});
"#,
        );
        test.result(result)
            .assert_lint("no-promise-executor-return")
            .assert_has_fix("no-promise-executor-return");
    }

    #[test]
    fn test_flags_executor_expression_body() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_flags_executor_expression_body.ds",
            r#"
let task = new Promise((resolve, reject) => resolve(1));
"#,
        );
        test.result(result)
            .assert_lint("no-promise-executor-return");
    }

    #[test]
    fn test_allows_executor_without_return() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_allows_executor_without_return.ds",
            r#"
let task = new Promise((resolve, reject) => {
    resolve(1);
});
"#,
        );
        test.result(result)
            .assert_no_lint("no-promise-executor-return");
    }

    #[test]
    fn test_fix_rewrites_executor_return_value() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_fix_rewrites_executor_return_value.ds",
            r#"
let task = new Promise((resolve, reject) => {
    return computeValue();
});
"#,
        );
        test.result(result)
            .assert_lint("no-promise-executor-return")
            .assert_unsafe_fixed(
                r#"
let task = new Promise((resolve, reject) => {
    {
        computeValue();
        return;
    }
});
"#,
            );
    }

    #[test]
    fn test_mutation_detects_multiple_executor_returns() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_mutation_detects_multiple_executor_returns.ds",
            r#"
let task = new Promise((resolve, reject) => {
    if (flag) {
        return computeA();
    }

    return computeB();
});
"#,
        );
        test.result(result)
            .assert_lint("no-promise-executor-return")
            .assert_unsafe_fixed(
                r#"
let task = new Promise((resolve, reject) => {
    if (flag) {
        {
            computeA();
            return;
        }
    }

    {
        computeB();
        return;
    }
});
"#,
            );
    }

    #[test]
    fn test_no_fix_for_executor_expression_body() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_no_fix_for_executor_expression_body.ds",
            r#"
let task = new Promise((resolve, reject) => resolve(1));
"#,
        );
        test.result(result)
            .assert_lint("no-promise-executor-return")
            .assert_has_no_fix("no-promise-executor-return");
    }

    #[test]
    fn test_allows_executor_return_without_value() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_allows_executor_return_without_value.ds",
            r#"
let task = new Promise((resolve, reject) => {
    reject(Error("failed"));
    return;
});
"#,
        );
        test.result(result)
            .assert_no_lint("no-promise-executor-return");
    }

    #[test]
    fn test_allows_nested_function_return_value() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_allows_nested_function_return_value.ds",
            r#"
let task = new Promise((resolve, reject) => {
    function helper() {
        return 1;
    }
    resolve(helper());
});
"#,
        );
        test.result(result)
            .assert_no_lint("no-promise-executor-return");
    }

    #[test]
    fn test_ignores_shadowed_promise_symbol() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_ignores_shadowed_promise_symbol.ds",
            r#"
function Promise(executor) {
    return executor;
}

let task = new Promise((resolve, reject) => {
    return 1;
});
"#,
        );
        test.result(result)
            .assert_no_lint("no-promise-executor-return");
    }

    #[test]
    fn test_ignores_promise_call_without_new() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_ignores_promise_call_without_new.ds",
            r#"
let task = Promise((resolve, reject) => {
    return 1;
});
"#,
        );
        test.result(result)
            .assert_no_lint("no-promise-executor-return");
    }

    #[test]
    fn test_flags_void_return_by_default() {
        let test = TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn);
        let result = test.lint_dir(
            "no_promise_executor_return/test_flags_void_return_by_default.ds",
            r#"
let task = new Promise((resolve, reject) => {
    return void resolve(1);
});
"#,
        );
        test.result(result)
            .assert_lint("no-promise-executor-return");
    }

    #[test]
    fn test_allows_void_return_when_configured() {
        let test =
            TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn).with_options(|options| {
                options.correctness.no_promise_executor_return_allow_void = true;
            });
        let result = test.lint_dir(
            "no_promise_executor_return/test_allows_void_return_when_configured.ds",
            r#"
let task = new Promise((resolve, reject) => {
    return void resolve(1);
});
"#,
        );
        test.result(result)
            .assert_no_lint("no-promise-executor-return");
    }

    #[test]
    fn test_allows_void_expression_body_when_configured() {
        let test =
            TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn).with_options(|options| {
                options.correctness.no_promise_executor_return_allow_void = true;
            });
        let result = test.lint_dir(
            "no_promise_executor_return/test_allows_void_expression_body_when_configured.ds",
            r#"
let task = new Promise((resolve, reject) => void resolve(1));
"#,
        );
        test.result(result)
            .assert_no_lint("no-promise-executor-return");
    }

    #[test]
    fn test_flags_mixed_void_and_value_returns_when_configured() {
        let test =
            TestProgram::for_rule_with_prelude(NoPromiseExecutorReturn).with_options(|options| {
                options.correctness.no_promise_executor_return_allow_void = true;
            });
        let result = test.lint_dir(
            "no_promise_executor_return/test_flags_mixed_void_and_value_returns_when_configured.ds",
            r#"
let task = new Promise((resolve, reject) => {
    if (flag) {
        return void resolve(1);
    }

    return computeValue();
});
"#,
        );
        test.result(result)
            .assert_lint("no-promise-executor-return");
    }
}
