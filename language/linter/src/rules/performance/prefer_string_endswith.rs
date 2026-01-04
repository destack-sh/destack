use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    const_i64, expression_target_symbol, is_string_type, string_literal_utf16_length,
    unwrap_parenthesized_expression,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `endsWith()` over `slice(-n) === suffix`.
    ///
    /// `endsWith()` communicates intent and avoids manual slicing.
    #[lint(
        id = "prefer-string-endswith",
        code = "LP021",
        category = Performance,
        level = Dir,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferStringEndsWith,
    "Prefer string.endsWith() over slice(-n) === suffix"
}

impl LintRule for PreferStringEndsWith {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferStringEndsWith::meta()
    }

    /// Check module DIR nodes for slice comparisons that should use endsWith().
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for endsWith comparisons
        let mut visitor = PreferStringEndsWithVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags prefer-string-endswith patterns.
struct PreferStringEndsWithVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known String symbol for this module.
    string_symbol: Option<dir::GlobalSymbolId>,
    /// The string id for the slice method name.
    slice_name: StringId,
    /// The string id for the length property name.
    length_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferStringEndsWithVisitor<'a, 'b> {
    /// Build a visitor for prefer-string-endswith checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // resolve the well known String symbol for this module
        let string_symbol = ctx.get_well_known_symbol(WellKnownSymbol::String);

        // intern commonly used names
        let slice_name = ctx.program.strings.intern("slice");
        let length_name = ctx.program.strings.intern("length");

        // prepare visitor state
        Self {
            ctx,
            meta,
            string_symbol,
            slice_name,
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

    /// Check comparison expressions for endsWith patterns.
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

        // match slice calls on the left
        if self.match_slice_comparison(left, right) {
            self.report_match(expression_id);
            return;
        }

        // match slice calls on the right
        if self.match_slice_comparison(right, left) {
            self.report_match(expression_id);
        }
    }

    /// Match a comparison with a slice call and suffix.
    fn match_slice_comparison(
        &mut self,
        slice_id: dir::LocalNodeId<dir::Expression>,
        suffix_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        // match call expression
        let expression = self.ctx.tree.get(slice_id);
        let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        else {
            return false;
        };
        let [argument_id] = dynamic_arguments.as_slice() else {
            return false;
        };
        let argument = self.ctx.tree.get(*argument_id);
        let argument_expression_id = argument.value();

        // match member access for slice
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return false;
        };
        if *name != self.slice_name {
            return false;
        }

        // ensure the receiver is a string
        if !self.is_string_receiver(*left) {
            return false;
        }

        // ensure the slice argument matches the suffix length
        self.is_suffix_length_match(argument_expression_id, suffix_id)
    }

    /// Report a prefer-string-endswith match.
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
                PREFER_STRING_ENDS_WITH.id,
                PREFER_STRING_ENDS_WITH.code,
                PREFER_STRING_ENDS_WITH.category,
                severity,
                "prefer endsWith() over slice(-n) comparison",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use endsWith() to check the suffix"),
        );
    }

    /// Return true when the receiver expression is a string type.
    fn is_string_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_string_type(self.ctx.types, type_id, self.string_symbol)
    }

    /// Return true when the slice argument matches the suffix length.
    fn is_suffix_length_match(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Expression>,
        suffix_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        // check for `-suffix.length` patterns
        if self.is_suffix_length_argument(argument_id, suffix_id) {
            return true;
        }

        // check for negative constant slice bounds with literal suffixes
        let Some(constant_value) = self.ctx.const_value(argument_id) else {
            return false;
        };
        let Some(constant) = const_i64(&constant_value) else {
            return false;
        };
        if constant >= 0 {
            return false;
        }

        let Some(suffix_length) = self.string_literal_length(suffix_id) else {
            return false;
        };

        let Some(absolute) = constant.checked_abs() else {
            return false;
        };

        suffix_length == absolute as usize
    }

    /// Return true when the argument is `-suffix.length`.
    fn is_suffix_length_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Expression>,
        suffix_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        // unwrap negation
        let argument_expression = self.ctx.tree.get(argument_id);
        let dir::Expression::Unary {
            operator: dir::UnaryOperator::Negate,
            right,
        } = argument_expression
        else {
            return false;
        };

        // match suffix.length member access
        let member_expression = self.ctx.tree.get(*right);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return false;
        };
        if *name != self.length_name {
            return false;
        }

        // compare suffix symbols when possible
        let Some(expected_symbol) = expression_target_symbol(self.ctx.tree, *left) else {
            return false;
        };
        let Some(actual_symbol) = expression_target_symbol(self.ctx.tree, suffix_id) else {
            return false;
        };

        expected_symbol == actual_symbol
    }

    /// Return the length for a string literal suffix.
    fn string_literal_length(&self, suffix_id: dir::LocalNodeId<dir::Expression>) -> Option<usize> {
        // unwrap parenthesized expressions
        let suffix_id = unwrap_parenthesized_expression(self.ctx.tree, suffix_id);

        // match string literals
        let expression = self.ctx.tree.get(suffix_id);
        let dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(value),
        } = expression
        else {
            return None;
        };

        Some(string_literal_utf16_length(
            &self.ctx.program.strings,
            *value,
        ))
    }
}

impl NodeVisitor for PreferStringEndsWithVisitor<'_, '_> {
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

    /// Report slice comparisons against suffix lengths.
    #[test]
    fn test_flags_slice_suffix_length_check() {
        let test = TestProgram::for_rule_without_builtins(PreferStringEndsWith);
        let result = test.lint(
            "test.ds",
            r#"
let text = "hello";
let suffix = "lo";
let ends = text.slice(-suffix.length) === suffix;
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("prefer-string-endswith");
    }

    /// Report literal slice comparisons.
    #[test]
    fn test_flags_literal_suffix_check() {
        let test = TestProgram::for_rule_without_builtins(PreferStringEndsWith);
        let result = test.lint(
            "test.ds",
            r#"
let text = "hello";
let ends = text.slice(-2) === "lo";
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_lint("prefer-string-endswith");
    }

    /// Allow unrelated slice comparisons.
    #[test]
    fn test_allows_unrelated_slice_check() {
        let test = TestProgram::for_rule_without_builtins(PreferStringEndsWith);
        let result = test.lint(
            "test.ds",
            r#"
let text = "hello";
let ends = text.slice(0) === "hello";
"#,
            LintLevel::Dir,
        );
        test.result(result).assert_no_lint("prefer-string-endswith");
    }
}
