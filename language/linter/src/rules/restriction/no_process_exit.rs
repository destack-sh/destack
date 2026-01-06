use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{expression_is_global_qualified_member, expression_target_symbol};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow process exit calls.
    ///
    /// Exiting the process abruptly can skip cleanup and is hard to test.
    #[lint(
        id = "no-process-exit",
        code = "LR025",
        category = Restriction,
        level = Dir,
        requires_all = [RequireLibSymbol("process", &["node"])],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoProcessExit,
    "Disallow process.exit usage"
}

impl LintRule for NoProcessExit {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoProcessExit::meta()
    }

    /// Check module DIR nodes for process exit calls.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for process exit calls
        let mut visitor = NoProcessExitVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags process.exit usage.
struct NoProcessExitVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The process symbol for this module.
    process_symbol: dir::GlobalSymbolId,
    /// The process member name.
    process_name: StringId,
    /// The exit member name.
    exit_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoProcessExitVisitor<'a, 'b> {
    /// Build a visitor for no-process-exit checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let process_name = ctx.program.strings.intern("process");
        let process_symbol = ctx.declared_lib_symbol(process_name);
        let exit_name = ctx.program.strings.intern("exit");
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            process_symbol,
            process_name,
            exit_name,
            global_qualifiers,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // walk the module expression tree
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call expression for process exit usage.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        // match member access
        let expression = self.ctx.tree.get(left);
        let dir::Expression::Member { left, name, .. } = expression else {
            return;
        };
        if *name != self.exit_name {
            return;
        }

        // require process receiver
        if !self.is_process_expression(*left) {
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
                NO_PROCESS_EXIT.id,
                NO_PROCESS_EXIT.code,
                NO_PROCESS_EXIT.category,
                severity,
                "process exit usage",
                self.ctx.module.file_id,
                span,
            )
            .with_label("avoid calling process.exit"),
        );
    }

    /// Return true when the expression refers to the process object.
    fn is_process_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match direct symbol references
        if let Some(symbol) = expression_target_symbol(self.ctx.tree, expression_id) {
            return symbol == self.process_symbol;
        }

        // match global qualified references
        expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.process_name,
        )
    }
}

impl NodeVisitor for NoProcessExitVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for process exit usage
        if let dir::Expression::Call { left, .. } = expression {
            self.check_call(id, *left);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report process exit calls.
    #[test]
    fn test_flags_process_exit_call() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "test.ds",
            r#"
process.exit(1);
"#,
        );
        test.result(result).assert_lint("no-process-exit");
    }

    /// Report global process exit calls.
    #[test]
    fn test_flags_global_process_exit_call() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "test.ds",
            r#"
globalThis.process.exit(1);
"#,
        );
        test.result(result).assert_lint("no-process-exit");
    }

    /// Allow other process calls.
    #[test]
    fn test_allows_other_process_call() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "test.ds",
            r#"
process.cwd();
"#,
        );
        test.result(result).assert_no_lint("no-process-exit");
    }
}
