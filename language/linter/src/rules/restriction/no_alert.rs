use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{expression_is_any_symbol, expression_is_standalone_statement};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow alert dialog browser calls.
    ///
    /// Alert, confirm, and prompt block execution and are usually undesirable.
    #[lint(
        id = "no-alert",
        code = "LR001",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [
            RequireLibSymbol("alert", &["dom"]),
            RequireLibSymbol("confirm", &["dom"]),
            RequireLibSymbol("prompt", &["dom"]),
        ],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable
    )]
    pub NoAlert,
    "Disallow alert, confirm, and prompt usage"
}

impl LintRule for NoAlert {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoAlert::meta()
    }

    /// Check module DIR nodes for alert dialog calls.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for alert calls
        let mut visitor = NoAlertVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags alert dialog usage.
struct NoAlertVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The alert symbol for this module.
    alert_symbol: Option<dir::GlobalSymbolId>,
    /// The confirm symbol for this module.
    confirm_symbol: Option<dir::GlobalSymbolId>,
    /// The prompt symbol for this module.
    prompt_symbol: Option<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoAlertVisitor<'a, 'b> {
    /// Build a visitor for no-alert checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let alert_name = ctx.string_id("alert");
        let confirm_name = ctx.string_id("confirm");
        let prompt_name = ctx.string_id("prompt");
        let alert_symbol = ctx.get_declared_library_symbol(alert_name);
        let confirm_symbol = ctx.get_declared_library_symbol(confirm_name);
        let prompt_symbol = ctx.get_declared_library_symbol(prompt_name);

        Self {
            ctx,
            meta,
            alert_symbol,
            confirm_symbol,
            prompt_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // skip when none of the alert symbols are available
        if self.alert_symbol.is_none()
            && self.confirm_symbol.is_none()
            && self.prompt_symbol.is_none()
        {
            return;
        }

        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        // walk the module expression tree
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call expression for alert dialog usage.
    fn check_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) {
        // resolve the callee symbol
        if !self.is_alert_expression(left) {
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
            NO_ALERT.id,
            NO_ALERT.code,
            NO_ALERT.category,
            severity,
            "alert dialog usage",
            span,
        )
        .label("avoid alert, confirm, and prompt calls");

        // compute fixes only when requested by the runner
        if self.ctx.compute_fixes
            && let Some(fix) = no_alert_fix(self.ctx, expression_id)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return true when the expression is an alert dialog reference.
    fn is_alert_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let symbols = [self.alert_symbol, self.confirm_symbol, self.prompt_symbol]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        expression_is_any_symbol(self.ctx, expression_id, &symbols)
    }
}

/// Build an unsafe fix by removing one standalone alert call statement.
fn no_alert_fix(
    ctx: &LintModuleContext<'_>,
    call_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    if !expression_is_standalone_statement(ctx.dir.tree(), call_id) {
        return None;
    }

    let statement_span = ctx.get_span(call_id);
    let edits = ctx.edit_builder().delete(statement_span).into_edits();
    Some(LintFix::r#unsafe("Remove alert dialog call").with_edits(edits))
}

impl NodeVisitor for NoAlertVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions for alert usage
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

    /// Report alert calls.
    #[test]
    fn test_flags_alert_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_flags_alert_call.ds",
            r#"
alert("stop");
"#,
        );
        test.result(result)
            .assert_lint("no-alert")
            .assert_has_fix("no-alert");
    }

    /// Report window confirm calls.
    #[test]
    fn test_flags_window_confirm_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_flags_window_confirm_call.ds",
            r#"
window.confirm("ok");
"#,
        );
        test.result(result).assert_lint("no-alert");
    }

    /// Report bracket-access alert calls on global qualifiers.
    #[test]
    fn test_flags_window_bracket_alert_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_flags_window_bracket_alert_call.ds",
            r#"
window["alert"]("stop");
"#,
        );
        test.result(result).assert_lint("no-alert");
    }

    /// Report bracket-access confirm calls on globalThis.
    #[test]
    fn test_flags_global_this_bracket_confirm_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_flags_global_this_bracket_confirm_call.ds",
            r#"
globalThis["confirm"]("ok");
"#,
        );
        test.result(result).assert_lint("no-alert");
    }

    /// Report direct alert calls on globalThis.
    #[test]
    fn test_flags_global_this_alert_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_flags_global_this_alert_call.ds",
            r#"
globalThis.alert("stop");
"#,
        );
        test.result(result).assert_lint("no-alert");
    }

    /// Report optional global alert calls.
    #[test]
    fn test_flags_optional_window_alert_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_flags_optional_window_alert_call.ds",
            r#"
window?.alert("stop");
"#,
        );
        test.result(result).assert_lint("no-alert");
    }

    /// Report prompt calls.
    #[test]
    fn test_flags_prompt_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_flags_prompt_call.ds",
            r#"
prompt("name");
"#,
        );
        test.result(result).assert_lint("no-alert");
    }

    /// Allow other function calls.
    #[test]
    fn test_allows_other_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_allows_other_call.ds",
            r#"
notify("ok");
"#,
        );
        test.result(result).assert_no_lint("no-alert");
    }

    /// Allow shadowed globalThis members.
    #[test]
    fn test_allows_shadowed_global_this_alert_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_allows_shadowed_global_this_alert_call.ds",
            r#"
const globalThis = {
    alert(value) {}
};

globalThis.alert("ok");
"#,
        );
        test.result(result).assert_no_lint("no-alert");
    }

    /// Allow bracket access on non-global objects.
    #[test]
    fn test_allows_non_global_bracket_alert_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_allows_non_global_bracket_alert_call.ds",
            r#"
const helper = {
    alert(value) {}
};

helper["alert"]("ok");
"#,
        );
        test.result(result).assert_no_lint("no-alert");
    }

    /// Unsafely remove standalone alert statements.
    #[test]
    fn test_fix_removes_alert_statement() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_fix_removes_alert_statement.ds",
            r#"
alert("stop");
"#,
        );
        test.result(result)
            .assert_lint("no-alert")
            .assert_unsafe_fixed(r#""#);
    }

    /// Unsafely remove standalone global-qualified confirm statements.
    #[test]
    fn test_fix_removes_global_confirm_statement() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_fix_removes_global_confirm_statement.ds",
            r#"
window.confirm("ok");
"#,
        );
        test.result(result)
            .assert_lint("no-alert")
            .assert_unsafe_fixed(r#""#);
    }

    /// Do not auto-fix alert calls when the result is used.
    #[test]
    fn test_no_fix_when_result_is_used() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_no_fix_when_result_is_used.ds",
            r#"
const accepted = confirm("ok");
"#,
        );
        test.result(result)
            .assert_lint("no-alert")
            .assert_has_no_fix("no-alert");
    }

    /// Mutation: detect prompt calls inside expression statements.
    #[test]
    fn test_mutation_detects_prompt_statement() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "no_alert/test_mutation_detects_prompt_statement.ds",
            r#"
prompt("name");
"#,
        );
        test.result(result)
            .assert_lint("no-alert")
            .assert_unsafe_fixed(r#""#);
    }
}
