use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{const_i64, flip_binary_operator, is_string_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `startsWith()` over `indexOf() === 0`.
    ///
    /// `startsWith()` communicates intent and avoids comparing against
    /// numeric sentinel values.
    #[lint(
        id = "prefer-string-startswith",
        code = "LP017",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::String)],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferStringStartsWith,
    "Prefer string.startsWith() over indexOf() === 0"
}

impl LintRule for PreferStringStartsWith {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferStringStartsWith::meta()
    }

    /// Check module DIR nodes for indexOf equality checks that should use startsWith().
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferStringStartsWithVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags prefer-string-startswith patterns.
struct PreferStringStartsWithVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// The string id for the indexOf method name.
    index_of_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferStringStartsWithVisitor<'a, 'b> {
    /// Build a visitor for prefer-string-startswith checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let string_symbol = ctx.well_known_symbol(WellKnownSymbol::String);
        let index_of_name = ctx.program.strings.intern("indexOf");

        Self {
            ctx,
            meta,
            string_symbol,
            index_of_name,
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

    /// Check comparison expressions for startsWith patterns.
    fn check_binary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        // match comparisons with the constant on the right
        if self.match_comparison(left, operator, right, false) {
            self.report_match(expression_id);
            return;
        }

        // match comparisons with the constant on the left
        if self.match_comparison(right, operator, left, true) {
            self.report_match(expression_id);
        }
    }

    /// Match a comparison with a candidate expression and constant.
    fn match_comparison(
        &mut self,
        candidate_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        constant_id: dir::LocalNodeId<dir::Expression>,
        flipped: bool,
    ) -> bool {
        // resolve constant comparisons
        let Some(constant_value) = self.ctx.const_value(constant_id) else {
            return false;
        };
        let Some(constant) = const_i64(&constant_value) else {
            return false;
        };

        // normalize operators when constants are on the left
        let operator = if flipped {
            let Some(operator) = flip_binary_operator(operator) else {
                return false;
            };
            operator
        } else {
            operator
        };

        // only match strict equality checks against zero
        if !matches!(
            operator,
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
        ) {
            return false;
        }
        if constant != 0 {
            return false;
        }

        self.is_index_of_call(candidate_id)
    }

    /// Report a prefer-string-startswith match.
    fn report_match(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                PREFER_STRING_STARTS_WITH.id,
                PREFER_STRING_STARTS_WITH.code,
                PREFER_STRING_STARTS_WITH.category,
                severity,
                "prefer startsWith() over indexOf() === 0",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use startsWith() to check the prefix"),
        );
    }

    /// Return true when the expression is a string indexOf call.
    fn is_index_of_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match call expression
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        else {
            return false;
        };
        if dynamic_arguments.is_empty() {
            return false;
        }

        // match member access for indexOf
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return false;
        };
        if *name != self.index_of_name {
            return false;
        }

        self.is_string_receiver(*left)
    }

    /// Return true when the receiver expression is a string type.
    fn is_string_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_string_type(self.ctx.types, type_id, Some(self.string_symbol))
    }
}

impl NodeVisitor for PreferStringStartsWithVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check comparison expressions
        if let dir::Expression::Binary {
            left,
            operator,
            right,
        } = expression
        {
            self.check_binary(id, *operator, *left, *right);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report indexOf checks against zero.
    #[test]
    fn test_flags_index_of_zero_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_flags_index_of_zero_check.ds",
            r#"
let text = "hello";
let has = text.indexOf("he") === 0;
"#,
        );
        test.result(result).assert_lint("prefer-string-startswith");
    }

    /// Allow non zero indexOf checks.
    #[test]
    fn test_allows_index_of_not_zero_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_allows_index_of_not_zero_check.ds",
            r#"
let text = "hello";
let has = text.indexOf("he") !== -1;
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-string-startswith");
    }
}
