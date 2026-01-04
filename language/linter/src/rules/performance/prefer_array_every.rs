use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{ReferencePath, expression_reference_path, is_array_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `every()` over `filter().length === array.length`.
    ///
    /// `every()` avoids allocating intermediate arrays and short circuits
    /// on the first failing element.
    #[lint(
        id = "prefer-array-every",
        code = "LP015",
        category = Performance,
        level = Dir,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArrayEvery,
    "Prefer array.every() over filter().length === array.length"
}

impl LintRule for PreferArrayEvery {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferArrayEvery::meta()
    }

    /// Check module DIR nodes for filter length comparisons that should use every().
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for array every comparisons
        let mut visitor = PreferArrayEveryVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags prefer-array-every patterns.
struct PreferArrayEveryVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: Option<dir::GlobalSymbolId>,
    /// The string id for the filter method name.
    filter_name: StringId,
    /// The string id for the length property name.
    length_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferArrayEveryVisitor<'a, 'b> {
    /// Build a visitor for prefer-array-every checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // resolve the well known Array symbol for this module
        let array_symbol = ctx.get_well_known_symbol(WellKnownSymbol::Array);

        // intern commonly used names
        let filter_name = ctx.program.strings.intern("filter");
        let length_name = ctx.program.strings.intern("length");

        // prepare visitor state
        Self {
            ctx,
            meta,
            array_symbol,
            filter_name,
            length_name,
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

    /// Check comparison expressions for prefer-array-every patterns.
    fn check_binary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        // only match equality comparisons
        if !matches!(
            operator,
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
        ) {
            return;
        }

        // match filter length on the left
        if let Some(receiver_path) = self.filter_length_receiver(left) {
            // confirm the lengths refer to the same receiver
            let is_same_length = self.is_same_array_length(&receiver_path, right);
            if is_same_length {
                self.report_match(expression_id);
                return;
            }
        }

        // match filter length on the right
        if let Some(receiver_path) = self.filter_length_receiver(right) {
            // confirm the lengths refer to the same receiver
            let is_same_length = self.is_same_array_length(&receiver_path, left);
            if is_same_length {
                self.report_match(expression_id);
            }
        }
    }

    /// Report a prefer-array-every match.
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
                PREFER_ARRAY_EVERY.id,
                PREFER_ARRAY_EVERY.code,
                PREFER_ARRAY_EVERY.category,
                severity,
                "prefer every() over filter().length comparison",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use array.every(...) to check if all elements match"),
        );
    }

    /// Return the receiver symbol for filter().length when it is an array filter.
    fn filter_length_receiver(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<ReferencePath> {
        // match `.length` member access
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Member { left, name, .. } = expression else {
            return None;
        };
        if *name != self.length_name {
            return None;
        }

        // match call expression on the left
        let call_id = *left;
        let call_expression = self.ctx.tree.get(call_id);
        let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = call_expression
        else {
            return None;
        };
        if dynamic_arguments.is_empty() {
            return None;
        }

        // match `.filter(...)` call
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return None;
        };
        if *name != self.filter_name {
            return None;
        }

        // resolve the receiver type
        let type_id = self.ctx.expression_type_id(*left)?;
        let is_array = is_array_type(self.ctx.types, type_id, self.array_symbol);
        if !is_array {
            return None;
        }

        // resolve the receiver path for matching
        expression_reference_path(self.ctx.tree, *left)
    }

    /// Return true when the expression is a matching array length.
    fn is_same_array_length(
        &mut self,
        receiver: &ReferencePath,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        // match `.length` member access
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Member { left, name, .. } = expression else {
            return false;
        };
        if *name != self.length_name {
            return false;
        }

        // resolve the receiver path
        let Some(path) = expression_reference_path(self.ctx.tree, *left) else {
            return false;
        };

        path == *receiver
    }
}

impl NodeVisitor for PreferArrayEveryVisitor<'_, '_> {
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
    use crate::LintLevel;
    use crate::linter::TestProgram;

    /// Report filter length comparisons against array length.
    #[test]
    fn test_flags_filter_length_equal_length() {
        let test = TestProgram::for_rule_without_builtins(PreferArrayEvery);
        let result = test.lint(
            "test.ds",
            r#"
let items = [1, 2, 3];
let all = items.filter(item => item > 1).length === items.length;
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("prefer-array-every");
    }

    /// Report reversed filter length comparisons.
    #[test]
    fn test_flags_reversed_filter_length() {
        let test = TestProgram::for_rule_without_builtins(PreferArrayEvery);
        let result = test.lint(
            "test.ds",
            r#"
let items = [1, 2, 3];
let all = items.length === items.filter(item => item > 1).length;
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("prefer-array-every");
    }

    /// Allow comparisons against unrelated lengths.
    #[test]
    fn test_allows_mismatched_arrays() {
        let test = TestProgram::for_rule_without_builtins(PreferArrayEvery);
        let result = test.lint(
            "test.ds",
            r#"
let items = [1, 2, 3];
let other = [1, 2, 3];
let all = items.filter(item => item > 1).length === other.length;
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_no_lint("prefer-array-every");
    }
}
