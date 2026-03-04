use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{expression_is_global_qualified_member, expression_target_symbol};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow console usage.
    ///
    /// Console statements are often left behind and should be removed.
    #[lint(
        id = "no-console",
        code = "LR007",
        category = Restriction,
        level = Dir,
        requires_all = [RequireLibSymbol("console", &["dom", "node"])],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable
    )]
    pub NoConsole,
    "Disallow console usage"
}

impl LintRule for NoConsole {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoConsole::meta()
    }

    /// Check module DIR nodes for console usage.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
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
    console_symbol: dir::GlobalSymbolId,
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
        let console_name = ctx.program.strings.intern("console");
        let console_symbol = ctx.declared_lib_symbol(console_name);
        let global_qualifiers = ctx.global_qualifier_symbols();

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
        let mut diagnostic = LintDiagnostic::new(
            NO_CONSOLE.id,
            NO_CONSOLE.code,
            NO_CONSOLE.category,
            severity,
            "console usage",
            self.ctx.module.file_id,
            span,
        )
        .with_label("remove console usage");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && let Some(fix) = no_console_fix(self.ctx, expression_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return true when the expression is a console reference.
    fn is_console_reference(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match direct symbol references
        if let Some(symbol) = expression_target_symbol(self.ctx.tree, expression_id) {
            return symbol == self.console_symbol;
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

/// Build an unsafe fix by removing one standalone console statement.
fn no_console_fix(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let parent = ctx.tree.get_parent(expression_id.id)?;
    if parent.ty != dir::NodeType::Expression {
        return None;
    }

    let parent_id = parent.into_typed::<dir::Expression>();
    let parent_expression = ctx.tree.get(parent_id);

    // remove `console...;` when the reported expression is directly statement scoped
    if let dir::Expression::Statement { statement } = parent_expression
        && *statement == expression_id
    {
        let statement_span = ctx.get_span(parent_id);
        let edits = ctx.edit_builder().delete(statement_span).into_edits();
        return Some(LintFix::r#unsafe("Remove console statement").with_edits(edits));
    }

    // remove call statements when the reported expression is the call callee
    let dir::Expression::Call { left, .. } = parent_expression else {
        return None;
    };
    if *left != expression_id {
        return None;
    }

    let grandparent = ctx.tree.get_parent(parent_id.id)?;
    if grandparent.ty != dir::NodeType::Expression {
        return None;
    }

    let grandparent_id = grandparent.into_typed::<dir::Expression>();
    let grandparent_expression = ctx.tree.get(grandparent_id);
    let dir::Expression::Statement { statement } = grandparent_expression else {
        return None;
    };
    if *statement != parent_id {
        return None;
    }

    let statement_span = ctx.get_span(grandparent_id);
    let edits = ctx.edit_builder().delete(statement_span).into_edits();
    Some(LintFix::r#unsafe("Remove console statement").with_edits(edits))
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

        // check computed console member usage
        if let dir::Expression::Index { left, .. } = expression {
            if self.is_console_reference(*left) {
                self.report_console(id);
            }

            // track index left to avoid double reporting
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
    use crate::linter::TestProgram;

    /// Report console method calls.
    #[test]
    fn test_flags_console_call() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_flags_console_call.ds",
            r#"
console.log("debug");
"#,
        );
        test.result(result)
            .assert_lint("no-console")
            .assert_has_fix("no-console");
    }

    /// Report global console usage.
    #[test]
    fn test_flags_window_console_call() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_flags_window_console_call.ds",
            r#"
globalThis.console.error("oops");
"#,
        );
        test.result(result)
            .assert_lint("no-console")
            .assert_has_fix("no-console");
    }

    /// Allow other member access.
    #[test]
    fn test_allows_other_member_access() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_allows_other_member_access.ds",
            r#"
logger.info("ok");
"#,
        );
        test.result(result).assert_no_lint("no-console");
    }

    /// Unsafely remove standalone console statements.
    #[test]
    fn test_fix_removes_console_statement() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_fix_removes_console_statement.ds",
            r#"
console.log("debug");
"#,
        );
        test.result(result)
            .assert_lint("no-console")
            .assert_unsafe_fixed(r#""#);
    }

    /// Unsafely remove standalone global console statements.
    #[test]
    fn test_fix_removes_global_console_statement() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_fix_removes_global_console_statement.ds",
            r#"
globalThis.console.error("oops");
"#,
        );
        test.result(result)
            .assert_lint("no-console")
            .assert_unsafe_fixed(r#""#);
    }

    /// Do not auto-fix console references when used as values.
    #[test]
    fn test_no_fix_when_console_value_is_used() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_no_fix_when_console_value_is_used.ds",
            r#"
const logRef = console.log;
"#,
        );
        test.result(result)
            .assert_lint("no-console")
            .assert_has_no_fix("no-console");
    }

    /// Mutation: detect method variants on console statements.
    #[test]
    fn test_mutation_detects_console_warn_statement() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_mutation_detects_console_warn_statement.ds",
            r#"
console.warn("warning");
"#,
        );
        test.result(result)
            .assert_lint("no-console")
            .assert_unsafe_fixed(r#""#);
    }

    /// Report computed console method calls.
    #[test]
    fn test_flags_computed_console_call() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_flags_computed_console_call.ds",
            r#"
console["log"]("debug");
"#,
        );
        test.result(result)
            .assert_lint("no-console")
            .assert_has_fix("no-console");
    }

    /// Report global computed console method calls.
    #[test]
    fn test_flags_global_computed_console_call() {
        let test = TestProgram::for_rule_with_prelude(NoConsole);
        let result = test.lint_dir(
            "no_console/test_flags_global_computed_console_call.ds",
            r#"
globalThis["console"]["warn"]("warning");
"#,
        );
        test.result(result)
            .assert_lint("no-console")
            .assert_has_fix("no-console");
    }
}
