use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{
    expression_is_global_qualified_member, expression_target_symbol, global_qualifier_symbols,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

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
        fixable = No,
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
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
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
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The alert symbol for this module.
    alert_symbol: Option<dir::GlobalSymbolId>,
    /// The confirm symbol for this module.
    confirm_symbol: Option<dir::GlobalSymbolId>,
    /// The prompt symbol for this module.
    prompt_symbol: Option<dir::GlobalSymbolId>,
    /// The alert member name.
    alert_name: StringId,
    /// The confirm member name.
    confirm_name: StringId,
    /// The prompt member name.
    prompt_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoAlertVisitor<'a, 'b> {
    /// Build a visitor for no-alert checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let alert_name = ctx.program.strings.intern("alert");
        let confirm_name = ctx.program.strings.intern("confirm");
        let prompt_name = ctx.program.strings.intern("prompt");
        let alert_symbol = ctx.get_declared_lib_symbol(alert_name);
        let confirm_symbol = ctx.get_declared_lib_symbol(confirm_name);
        let prompt_symbol = ctx.get_declared_lib_symbol(prompt_name);
        let global_qualifiers = global_qualifier_symbols(ctx);

        Self {
            ctx,
            meta,
            alert_symbol,
            confirm_symbol,
            prompt_symbol,
            alert_name,
            confirm_name,
            prompt_name,
            global_qualifiers,
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
        let tree = self.ctx.tree;

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
        self.ctx.report(
            LintDiagnostic::new(
                NO_ALERT.id,
                NO_ALERT.code,
                NO_ALERT.category,
                severity,
                "alert dialog usage",
                self.ctx.module.file_id,
                span,
            )
            .with_label("avoid alert, confirm, and prompt calls"),
        );
    }

    /// Return true when the symbol is an alert dialog global.
    fn is_alert_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        let symbols = [self.alert_symbol, self.confirm_symbol, self.prompt_symbol];
        symbols
            .into_iter()
            .flatten()
            .any(|alert_symbol| alert_symbol == symbol)
    }

    /// Return true when the expression is an alert dialog reference.
    fn is_alert_expression(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match direct symbol references
        if let Some(symbol) = expression_target_symbol(self.ctx.tree, expression_id) {
            return self.is_alert_symbol(symbol);
        }

        // match global qualified references
        self.is_global_alert(expression_id)
    }

    /// Return true when the expression is a global qualified alert.
    fn is_global_alert(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // check global qualified alert references
        let is_alert = expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.alert_name,
        );

        // check global qualified confirm references
        let is_confirm = expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.confirm_name,
        );

        // check global qualified prompt references
        let is_prompt = expression_is_global_qualified_member(
            self.ctx.tree,
            expression_id,
            &self.global_qualifiers,
            self.prompt_name,
        );

        is_alert || is_confirm || is_prompt
    }
}

impl NodeVisitor for NoAlertVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
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
            "test.ds",
            r#"
alert("stop");
"#,
        );
        test.result(result).assert_lint("no-alert");
    }

    /// Report window confirm calls.
    #[test]
    fn test_flags_window_confirm_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "test.ds",
            r#"
window.confirm("ok");
"#,
        );
        test.result(result).assert_lint("no-alert");
    }

    /// Report prompt calls.
    #[test]
    fn test_flags_prompt_call() {
        let test = TestProgram::for_rule_with_prelude(NoAlert);
        let result = test.lint_dir(
            "test.ds",
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
            "test.ds",
            r#"
notify("ok");
"#,
        );
        test.result(result).assert_no_lint("no-alert");
    }
}
