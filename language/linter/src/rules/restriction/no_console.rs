use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_is_global_qualified_member, expression_target_symbol, global_qualifier_symbols,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow console usage.
    ///
    /// Console statements are often left behind and should be removed.
    #[lint(
        id = "no-console",
        code = "LR008",
        category = Restriction,
        level = Dir,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoConsole,
    "Disallow console usage"
}

// FUGU: ensure new DIR linter rules still work (after Analyze changes)

impl LintRule for NoConsole {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoConsole::meta()
    }

    /// Check module DIR nodes for console usage.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for console usage
        let mut visitor = NoConsoleVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags console usage.
struct NoConsoleVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The console symbol for this module.
    console_symbol: Option<dir::GlobalSymbolId>,
    /// The console member name.
    console_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// Stack of member left expressions to avoid double reporting.
    member_left_stack: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoConsoleVisitor<'a, 'b> {
    /// Build a visitor for no-console checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // intern commonly used names
        let console_name = ctx.program.strings.intern("console");

        // resolve lib symbols
        let console_symbol = ctx.get_lib_item(console_name);
        let global_qualifiers = global_qualifier_symbols(ctx);

        // prepare visitor state
        Self {
            ctx,
            meta,
            console_symbol,
            console_name,
            global_qualifiers,
            member_left_stack: Vec::new(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // skip when console is unavailable
        if self.console_symbol.is_none() {
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

    /// Report a console diagnostic.
    fn report_console(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_CONSOLE.id,
                NO_CONSOLE.code,
                NO_CONSOLE.category,
                severity,
                "console usage",
                self.ctx.module.file_id,
                span,
            )
            .with_label("remove console usage"),
        );
    }

    /// Return true when the expression is a console reference.
    fn is_console_reference(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match direct symbol references
        if let Some(symbol) = expression_target_symbol(self.ctx.tree, expression_id) {
            return self
                .console_symbol
                .is_some_and(|console_symbol| console_symbol == symbol);
        }

        // match global qualified references
        expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.console_name,
        )
    }

    /// Return true when the expression is the left side of a member access.
    #[inline]
    fn is_member_left(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        self.member_left_stack.contains(&expression_id)
    }
}

impl NodeVisitor for NoConsoleVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check console member usage
        if let dir::Expression::Member { left, .. } = expression {
            if self.is_console_reference(*left) {
                self.report_console(id);
            }

            // track member left to avoid double reporting
            self.member_left_stack.push(*left);
            walk_expression(self, tree, id, expression);
            self.member_left_stack.pop();
            return;
        }

        // check direct console references
        if self.is_console_reference(id) && !self.is_member_left(id) {
            self.report_console(id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LintLevel;
    use crate::linter::TestProgram;
    use destack_workspace::Runtime;

    /// Report console method calls.
    #[test]
    fn test_flags_console_call() {
        let test = TestProgram::for_rule_with_builtins(NoConsole)
            .with_runtime(Runtime::Node)
            .with_lib("node");
        let result = test.lint(
            "test.ds",
            r#"
console.log("debug");
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("no-console");
    }

    /// Report global console usage.
    #[test]
    fn test_flags_window_console_call() {
        let test = TestProgram::for_rule_with_builtins(NoConsole)
            .with_runtime(Runtime::Node)
            .with_lib("node");
        let result = test.lint(
            "test.ds",
            r#"
globalThis.console.error("oops");
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("no-console");
    }

    /// Allow other member access.
    #[test]
    fn test_allows_other_member_access() {
        let test = TestProgram::for_rule_with_builtins(NoConsole)
            .with_runtime(Runtime::Node)
            .with_lib("node");
        let result = test.lint(
            "test.ds",
            r#"
logger.info("ok");
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_no_lint("no-console");
    }
}
