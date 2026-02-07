use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_target_symbol;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow functions that unconditionally call themselves.
    ///
    /// A function that calls itself without any conditional guard will recurse
    /// infinitely and cause a stack overflow.
    #[lint(
        id = "no-infinite-recursion",
        code = "LC028",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoInfiniteRecursion,
    "Disallow infinite recursion"
}

impl LintRule for NoInfiniteRecursion {
    fn meta(&self) -> &'static LintMeta {
        NoInfiniteRecursion::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let module_id = ctx.module.id;

        // collect functions to check (avoid borrowing ctx during iteration)
        let functions: Vec<_> = ctx
            .tree
            .iter_nodes_of_type::<dir::Declaration>()
            .filter_map(|(decl_id, decl)| {
                if let dir::Declaration::Function {
                    descriptor,
                    body: Some(body_id),
                    ..
                } = decl
                {
                    let function_symbol = dir::GlobalSymbolId::new(module_id, descriptor.symbol);
                    Some((decl_id, function_symbol, *body_id))
                } else {
                    None
                }
            })
            .collect();

        // check each function
        for (decl_id, function_symbol, body_id) in functions {
            let mut visitor = InfiniteRecursionVisitor::new(ctx, meta, function_symbol, decl_id);
            visitor.run(body_id);
        }
    }
}

/// Visitor that detects unconditional recursive calls.
struct InfiniteRecursionVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The function's own symbol.
    function_symbol: dir::GlobalSymbolId,
    /// The declaration node id for span reporting.
    decl_id: dir::LocalNodeId<dir::Declaration>,
    /// Whether we've seen any conditional (if/match).
    has_conditional: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> InfiniteRecursionVisitor<'a, 'b> {
    /// Build a new visitor.
    fn new(
        ctx: &'a mut LintModuleDirContext<'b>,
        meta: &'a LintMeta,
        function_symbol: dir::GlobalSymbolId,
        decl_id: dir::LocalNodeId<dir::Declaration>,
    ) -> Self {
        Self {
            ctx,
            meta,
            function_symbol,
            decl_id,
            has_conditional: false,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the function body.
    fn run(&mut self, body_id: dir::LocalNodeId<dir::Expression>) {
        let tree = self.ctx.tree;
        let body = tree.get(body_id);
        self.visit_expression(tree, body_id, body);
    }

    /// Check if a call expression is a recursive call.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        // if we've seen any conditional, assume the recursion might be guarded
        if self.has_conditional {
            return;
        }

        // check if the call target is the current function
        let Some(target_symbol) = expression_target_symbol(self.ctx.tree, left) else {
            return;
        };
        if target_symbol != self.function_symbol {
            return;
        }

        // check effective severity
        let severity = self.ctx.get_effective_severity(self.meta, self.decl_id);
        if !severity.is_enabled() {
            return;
        }

        // report
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_INFINITE_RECURSION.id,
                NO_INFINITE_RECURSION.code,
                NO_INFINITE_RECURSION.category,
                severity,
                "function unconditionally calls itself",
                self.ctx.module.file_id,
                span,
            )
            .with_label("this recursive call has no base case"),
        );
    }
}

impl NodeVisitor for InfiniteRecursionVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // track if we've seen any conditional control flow
        if matches!(
            expression,
            dir::Expression::If { .. } | dir::Expression::Match { .. }
        ) {
            self.has_conditional = true;
        }

        // check call expressions
        if let dir::Expression::Call { left, .. } = expression {
            self.check_call(id, *left);
        }

        // walk children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag function that unconditionally calls itself.
    #[test]
    fn test_flags_direct_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_flags_direct_recursion.ds",
            r#"
function infinite() {
    infinite();
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-infinite-recursion");
    }

    /// Flag function that unconditionally calls itself with arguments.
    #[test]
    fn test_flags_recursion_with_args() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_flags_recursion_with_args.ds",
            r#"
function process(x: number) {
    process(x + 1);
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-infinite-recursion");
    }

    /// Allow recursion guarded by if statement.
    #[test]
    fn test_allows_conditional_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_conditional_recursion.ds",
            r#"
function factorial(n: number): number {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }

    /// Allow recursion guarded by match.
    #[test]
    fn test_allows_match_recursion() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_match_recursion.ds",
            r#"
function count(n: number): number {
    match (n) {
        0 => 0
        _ => 1 + count(n - 1)
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }

    /// Allow calling a different function.
    #[test]
    fn test_allows_different_function() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_different_function.ds",
            r#"
function helper(x: number): number {
    return x * 2;
}

function process(x: number): number {
    return helper(x);
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }

    /// Allow non-recursive functions.
    #[test]
    fn test_allows_non_recursive() {
        let test = TestProgram::for_rule_without_prelude(NoInfiniteRecursion);
        let result = test.lint_dir(
            "no_infinite_recursion/test_allows_non_recursive.ds",
            r#"
function add(a: number, b: number): number {
    return a + b;
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-infinite-recursion");
    }
}
