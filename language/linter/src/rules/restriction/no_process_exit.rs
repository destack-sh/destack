use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{expression_is_global_qualified_member, expression_target_symbol};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow process exit calls.
    ///
    /// Exiting the process abruptly can skip cleanup and is hard to test.
    #[lint(
        id = "no-process-exit",
        code = "LR023",
        category = Restriction,
        level = Dir,
        requires_all = [RequireLibSymbol("process", &["node"])],
        requires_any = [],
        fixable = Sometimes,
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
        let mut diagnostic = LintDiagnostic::new(
            NO_PROCESS_EXIT.id,
            NO_PROCESS_EXIT.code,
            NO_PROCESS_EXIT.category,
            severity,
            "process exit usage",
            self.ctx.module.file_id,
            span,
        )
        .with_label("avoid calling process.exit");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && let Some(fix) = no_process_exit_fix(self.ctx, expression_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
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

/// Build an unsafe fix by removing one standalone process exit statement.
fn no_process_exit_fix(
    ctx: &LintModuleDirContext<'_>,
    call_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let parent = ctx.tree.get_parent(call_id.id)?;
    if parent.ty != dir::NodeType::Expression {
        return None;
    }

    let parent_id = parent.into_typed::<dir::Expression>();
    let parent_expression = ctx.tree.get(parent_id);
    let dir::Expression::Statement { statement } = parent_expression else {
        return None;
    };
    if *statement != call_id {
        return None;
    }

    let statement_span = ctx.get_span(parent_id);
    let edits = ctx.edit_builder().delete(statement_span).into_edits();
    Some(LintFix::r#unsafe("Remove process.exit statement").with_edits(edits))
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
            "no_process_exit/test_flags_process_exit_call.ds",
            r#"
process.exit(1);
"#,
        );
        test.result(result)
            .assert_lint("no-process-exit")
            .assert_has_fix("no-process-exit");
    }

    /// Report global process exit calls.
    #[test]
    fn test_flags_global_process_exit_call() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_flags_global_process_exit_call.ds",
            r#"
globalThis.process.exit(1);
"#,
        );
        test.result(result)
            .assert_lint("no-process-exit")
            .assert_has_fix("no-process-exit");
    }

    /// Allow other process calls.
    #[test]
    fn test_allows_other_process_call() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_allows_other_process_call.ds",
            r#"
process.cwd();
"#,
        );
        test.result(result).assert_no_lint("no-process-exit");
    }

    /// Unsafely remove standalone process.exit statements.
    #[test]
    fn test_fix_removes_process_exit_statement() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_fix_removes_process_exit_statement.ds",
            r#"
process.exit(1);
"#,
        );
        test.result(result)
            .assert_lint("no-process-exit")
            .assert_unsafe_fixed(r#""#);
    }

    /// Unsafely remove standalone global process.exit statements.
    #[test]
    fn test_fix_removes_global_process_exit_statement() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_fix_removes_global_process_exit_statement.ds",
            r#"
globalThis.process.exit(1);
"#,
        );
        test.result(result)
            .assert_lint("no-process-exit")
            .assert_unsafe_fixed(r#""#);
    }

    /// Do not auto-fix process.exit values when used in expressions.
    #[test]
    fn test_no_fix_when_process_exit_result_is_used() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_no_fix_when_process_exit_result_is_used.ds",
            r#"
const status = process.exit(1);
"#,
        );
        test.result(result)
            .assert_lint("no-process-exit")
            .assert_has_no_fix("no-process-exit");
    }

    /// Mutation: detect zero-status process.exit statements.
    #[test]
    fn test_mutation_detects_process_exit_zero_status() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_mutation_detects_process_exit_zero_status.ds",
            r#"
process.exit(0);
"#,
        );
        test.result(result)
            .assert_lint("no-process-exit")
            .assert_unsafe_fixed(r#""#);
    }
}
