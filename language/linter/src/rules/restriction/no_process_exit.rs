use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{
    expression_is_symbol_or_global_qualified_member, expression_static_property_access,
    statement_expression_ancestor,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

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
        let meta = self.meta();

        // cli entrypoints often exit deliberately
        if ctx.file.has_hashbang() {
            return;
        }

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
    /// The process event registration method names.
    event_handler_names: [StringId; 2],
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoProcessExitVisitor<'a, 'b> {
    /// Build a visitor for no-process-exit checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let process_name = ctx.string_id("process");
        let process_symbol = ctx.declared_library_symbol(process_name);
        let exit_name = ctx.string_id("exit");
        let event_handler_names = [ctx.string_id("on"), ctx.string_id("once")];
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            process_symbol,
            process_name,
            exit_name,
            event_handler_names,
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
        // match static property access
        let Some((receiver_id, property_name)) =
            expression_static_property_access(self.ctx.tree, left)
        else {
            return;
        };
        if property_name != self.exit_name {
            return;
        }

        // require process receiver
        if !self.is_process_expression(receiver_id) {
            return;
        }
        if self.is_inside_process_event_handler_callback(expression_id) {
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
            NO_PROCESS_EXIT.id,
            NO_PROCESS_EXIT.code,
            NO_PROCESS_EXIT.category,
            severity,
            "process exit usage",
            span,
        )
        .label("avoid calling process.exit");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && let Some(fix) = no_process_exit_fix(self.ctx, expression_id)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return true when the expression refers to the process object.
    fn is_process_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        expression_is_symbol_or_global_qualified_member(
            self.ctx,
            expression_id,
            self.process_symbol,
            &self.global_qualifiers,
            self.process_name,
        )
    }

    /// Return true when one call target is `process.on(...)` or `process.once(...)`.
    fn is_process_event_handler_registration(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let Some((receiver_id, property_name)) =
            expression_static_property_access(self.ctx.tree, expression_id)
        else {
            return false;
        };
        if !self.event_handler_names.contains(&property_name) {
            return false;
        }

        self.is_process_expression(receiver_id)
    }

    /// Return true when one process.exit call is inside a process event callback argument.
    fn is_inside_process_event_handler_callback(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let mut current_id = expression_id.id;

        while let Some(parent_id) = self.ctx.tree.get_parent(current_id) {
            current_id = parent_id.id;
            if parent_id.ty != dir::NodeType::Argument {
                continue;
            }

            let argument_id = parent_id.into_typed::<dir::Argument>();
            if !argument_is_function_like(self.ctx.tree, argument_id) {
                continue;
            }

            let Some(call_parent_id) = self.ctx.tree.get_parent(argument_id.id) else {
                continue;
            };
            if call_parent_id.ty != dir::NodeType::Expression {
                continue;
            }

            let call_id = call_parent_id.into_typed::<dir::Expression>();
            let dir::Expression::Call { left, .. } = self.ctx.tree.get(call_id) else {
                continue;
            };
            if self.is_process_event_handler_registration(*left) {
                return true;
            }
        }

        false
    }
}

/// Return true when one argument value is a function-like expression.
fn argument_is_function_like(
    tree: &dir::Tree,
    argument_id: dir::LocalNodeId<dir::Argument>,
) -> bool {
    let argument = tree.get(argument_id);
    expression_is_function_like(tree, argument.value())
}

/// Return true when one expression is a function declaration expression.
fn expression_is_function_like(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let dir::Expression::Declaration(declaration) = tree.get(expression_id) else {
        return false;
    };

    matches!(tree.get(*declaration), dir::Declaration::Function(_))
}

/// Build an unsafe fix by removing one standalone process exit statement.
fn no_process_exit_fix(
    ctx: &LintModuleDirContext<'_>,
    call_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let statement_id = statement_expression_ancestor(ctx.tree, call_id)?;
    let statement_span = ctx.get_span(statement_id);
    let edits = ctx.edit_builder().delete(statement_span).into_edits();
    Some(LintFix::r#unsafe("Remove process.exit statement").with_edits(edits))
}

impl NodeVisitor for NoProcessExitVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
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

    /// Report computed process exit calls.
    #[test]
    fn test_flags_computed_process_exit_call() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_flags_computed_process_exit_call.ds",
            r#"
process["exit"](1);
"#,
        );
        test.result(result)
            .assert_lint("no-process-exit")
            .assert_has_fix("no-process-exit");
    }

    /// Report global computed process exit calls.
    #[test]
    fn test_flags_global_computed_process_exit_call() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_flags_global_computed_process_exit_call.ds",
            r#"
globalThis.process["exit"](1);
"#,
        );
        test.result(result)
            .assert_lint("no-process-exit")
            .assert_has_fix("no-process-exit");
    }

    /// Allow local shadowing of process.
    #[test]
    fn test_allows_shadowed_local_process_exit_call() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_allows_shadowed_local_process_exit_call.ds",
            r#"
const process = {
    exit(code: int32): void {
        log(code);
    },
};

process.exit(1);
"#,
        );
        test.result(result).assert_no_lint("no-process-exit");
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

    /// Allow process exit in CLI entrypoints.
    #[test]
    fn test_allows_process_exit_in_hashbang_entrypoint() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_allows_process_exit_in_hashbang_entrypoint.ds",
            r#"#!/usr/bin/env node
process.exit(1);
"#,
        );
        test.result(result).assert_no_lint("no-process-exit");
    }

    /// Allow process exit inside process event handlers.
    #[test]
    fn test_allows_process_exit_in_process_on_handler() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_allows_process_exit_in_process_on_handler.ds",
            r#"
process.on("SIGINT", (): void => {
    process.exit(1);
});
"#,
        );
        test.result(result).assert_no_lint("no-process-exit");
    }

    /// Allow process exit inside process once handlers.
    #[test]
    fn test_allows_process_exit_in_process_once_handler() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_allows_process_exit_in_process_once_handler.ds",
            r#"
process.once("SIGTERM", (): void => {
    process.exit(1);
});
"#,
        );
        test.result(result).assert_no_lint("no-process-exit");
    }

    /// Keep reporting process exit outside the actual event callback body.
    #[test]
    fn test_flags_process_exit_in_process_on_non_callback_argument() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_flags_process_exit_in_process_on_non_callback_argument.ds",
            r#"
process.on(process.exit(1), (): void => {});
"#,
        );
        test.result(result).assert_lint("no-process-exit");
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

    /// Unsafely remove parenthesized standalone process.exit statements.
    #[test]
    fn test_fix_removes_parenthesized_process_exit_statement() {
        let test = TestProgram::for_rule_with_prelude(NoProcessExit);
        let result = test.lint_dir(
            "no_process_exit/test_fix_removes_parenthesized_process_exit_statement.ds",
            r#"
(process.exit(1));
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

    /// Mutation: detect zero status process.exit statements.
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
