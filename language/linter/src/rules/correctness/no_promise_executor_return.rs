use destack_builtin::WellKnownSymbol;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_target_symbol;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow returning values from Promise executors.
    ///
    /// Promise executors should call resolve or reject instead of returning values.
    #[lint(
        id = "no-promise-executor-return",
        code = "LC037",
        category = Correctness,
        level = Dir,
        fixable = No,
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
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for promise executor returns
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
    promise_symbol: Option<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PromiseExecutorReturnVisitor<'a, 'b> {
    /// Build a visitor for Promise executor return checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // resolve the well known Promise symbol for this module
        let promise_symbol = ctx.get_well_known_symbol(WellKnownSymbol::Promise);

        // prepare visitor state
        Self {
            ctx,
            meta,
            promise_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // skip when no Promise symbol is available
        if self.promise_symbol.is_none() {
            return;
        }

        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // walk the module expression tree
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
        dynamic_arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // ignore non promise calls
        let Some(promise_symbol) = self.promise_symbol else {
            return;
        };
        let Some(target_symbol) = expression_target_symbol(self.ctx.tree, left) else {
            return;
        };
        if target_symbol != promise_symbol {
            return;
        }

        // get the executor argument
        let Some(argument_id) = dynamic_arguments.first() else {
            return;
        };
        let argument = self.ctx.tree.get(*argument_id);
        let value_id = argument.value();

        // resolve the executor declaration
        let Some(declaration_id) = executor_declaration(self.ctx, value_id) else {
            return;
        };
        if !executor_returns_value(self.ctx.tree, declaration_id) {
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
                NO_PROMISE_EXECUTOR_RETURN.id,
                NO_PROMISE_EXECUTOR_RETURN.code,
                NO_PROMISE_EXECUTOR_RETURN.category,
                severity,
                "avoid returning values from Promise executors",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use resolve or reject instead of returning"),
        );
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
        // check Promise calls and constructors
        match expression {
            dir::Expression::Call {
                left,
                dynamic_arguments,
                ..
            } => self.check_executor_returns(id, *left, dynamic_arguments),
            dir::Expression::New {
                left,
                dynamic_arguments,
                ..
            } => self.check_executor_returns(id, *left, dynamic_arguments),
            _ => {}
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
    if let dir::Expression::Declaration { declaration } = expression {
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
fn executor_returns_value(
    tree: &dir::NodeTree,
    declaration_id: dir::LocalNodeId<dir::Declaration>,
) -> bool {
    // extract the function body
    let declaration = tree.get(declaration_id);
    let dir::Declaration::Function {
        signature, body, ..
    } = declaration
    else {
        return false;
    };

    let Some(body_id) = body else {
        return false;
    };

    // treat expression bodies as implicit returns for lambdas
    let body_expression = tree.get(*body_id);
    if signature.kind == dir::FunctionKind::Lambda
        && !matches!(body_expression, dir::Expression::Block { .. })
    {
        return true;
    }

    // walk the body for explicit returns with values
    let mut visitor = ReturnValueVisitor::default();
    visitor.visit_expression(tree, *body_id, body_expression);
    visitor.found_return_value
}

/// Visitor that checks for return values.
#[derive(Default)]
struct ReturnValueVisitor {
    /// Whether a return value was found.
    found_return_value: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl NodeVisitor for ReturnValueVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // short circuit once a value return is found
        if self.found_return_value {
            return;
        }

        // detect explicit return values
        if let dir::Expression::Return { value } = expression
            && value.is_some()
        {
            self.found_return_value = true;
            return;
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }

    fn visit_declaration(
        &mut self,
        _tree: &dir::NodeTree,
        _id: dir::LocalNodeId<dir::Declaration>,
        _declaration: &dir::Declaration,
    ) {
        // skip nested function declarations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LintLevel;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_executor_return_value() {
        let test = TestProgram::for_rule_with_builtins(NoPromiseExecutorReturn);
        let result = test.lint(
            "test.ds",
            r#"
let task = new Promise((resolve, reject) => {
    return 1;
});
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-promise-executor-return");
    }

    #[test]
    fn test_flags_executor_expression_body() {
        let test = TestProgram::for_rule_with_builtins(NoPromiseExecutorReturn);
        let result = test.lint(
            "test.ds",
            r#"
let task = new Promise((resolve, reject) => resolve(1));
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-promise-executor-return");
    }

    #[test]
    fn test_allows_executor_without_return() {
        let test = TestProgram::for_rule_with_builtins(NoPromiseExecutorReturn);
        let result = test.lint(
            "test.ds",
            r#"
let task = new Promise((resolve, reject) => {
    resolve(1);
});
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("no-promise-executor-return");
    }
}
