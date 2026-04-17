use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    ReferencePath, expression_reference_path, expression_unwrap_parenthesized, is_array_type,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `every()` over `filter().length === array.length`.
    ///
    /// `every()` avoids allocating intermediate arrays and short circuits
    /// on the first failing element.
    #[lint(
        id = "prefer-array-every",
        code = "LP012",
        category = Performance,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
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
        let meta = self.meta();
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
    array_symbol: dir::GlobalSymbolId,
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
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let filter_name = ctx.repository.strings.intern("filter");
        let length_name = ctx.repository.strings.intern("length");

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
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

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
        let left = expression_unwrap_parenthesized(self.ctx.tree, left);
        let right = expression_unwrap_parenthesized(self.ctx.tree, right);

        // match filter length on the left
        if let Some(filter_match) = self.filter_length_match(left) {
            // confirm the lengths refer to the same receiver
            let is_same_length = self.is_same_array_length(&filter_match.receiver_path, right);
            if is_same_length && operator_implies_all_match(operator, true) {
                self.report_match(expression_id, &filter_match);
                return;
            }
        }

        // match filter length on the right
        if let Some(filter_match) = self.filter_length_match(right) {
            // confirm the lengths refer to the same receiver
            let is_same_length = self.is_same_array_length(&filter_match.receiver_path, left);
            if is_same_length && operator_implies_all_match(operator, false) {
                self.report_match(expression_id, &filter_match);
            }
        }
    }

    /// Report a prefer-array-every match.
    fn report_match(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        filter_match: &FilterLengthMatch,
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_ARRAY_EVERY.id,
            PREFER_ARRAY_EVERY.code,
            PREFER_ARRAY_EVERY.category,
            severity,
            "prefer every() over filter().length comparison",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use array.every(...) to check if all elements match");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && let Some(fix) = self.prefer_array_every_fix(expression_id, filter_match)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return filter match data for filter().length when it is an array filter.
    fn filter_length_match(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<FilterLengthMatch> {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);

        // match `.length` member access
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Member { left, name, .. } = expression else {
            return None;
        };
        if *name != Some(self.length_name) {
            return None;
        }

        // match call expression on the left
        let call_id = *left;
        let call_expression = self.ctx.tree.get(call_id);
        let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
            ..
        } = call_expression
        else {
            return None;
        };
        if arguments.is_empty() {
            return None;
        }

        // match `.filter(...)` call
        let member_id = *left;
        let member_expression = self.ctx.tree.get(member_id);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return None;
        };
        if *name != Some(self.filter_name) {
            return None;
        }

        // resolve the array receiver expression before `.filter`
        let receiver_id = *left;

        // resolve the receiver type
        let type_id = self.ctx.expression_type_id(receiver_id)?;
        let is_array = is_array_type(self.ctx.types, type_id, Some(self.array_symbol));
        if !is_array {
            return None;
        }

        // resolve the receiver path for matching
        let receiver_path = expression_reference_path(self.ctx.tree, receiver_id)?;
        Some(FilterLengthMatch {
            receiver_path,
            arguments: arguments.clone(),
            has_generic_arguments: !generic_arguments.is_empty(),
        })
    }

    /// Return true when the expression is a matching array length.
    fn is_same_array_length(
        &mut self,
        receiver: &ReferencePath,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);

        // match `.length` member access
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Member { left, name, .. } = expression else {
            return false;
        };
        if *name != Some(self.length_name) {
            return false;
        }

        // resolve the receiver path
        let Some(path) = expression_reference_path(self.ctx.tree, *left) else {
            return false;
        };

        path == *receiver
    }

    /// Build an unsafe fix that rewrites filter().length checks to every().
    fn prefer_array_every_fix(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        filter_match: &FilterLengthMatch,
    ) -> Option<LintFix> {
        if filter_match.has_generic_arguments {
            return None;
        }

        // resolve the array-length side from the matched comparison
        let binary_expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Binary { left, right, .. } = binary_expression else {
            return None;
        };
        let receiver_text = if self.is_same_array_length(&filter_match.receiver_path, *left) {
            self.array_receiver_text_from_length(*left)?
        } else if self.is_same_array_length(&filter_match.receiver_path, *right) {
            self.array_receiver_text_from_length(*right)?
        } else {
            return None;
        };

        // ensure receiver text exists
        if receiver_text.trim().is_empty() {
            return None;
        }

        if filter_match.arguments.is_empty() {
            return None;
        }

        let mut argument_texts = Vec::new();
        for argument_id in &filter_match.arguments {
            let argument_span = self.ctx.get_span(*argument_id);
            let argument_text = self.ctx.get_span_text(argument_span);
            if argument_text.trim().is_empty() {
                return None;
            }
            argument_texts.push(argument_text.to_string());
        }

        let arguments = argument_texts.join(", ");
        let replacement = format!("{receiver_text}.every({arguments})");
        let span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(span, replacement)
            .into_edits();
        Some(LintFix::r#unsafe("Replace filter().length check with every()").with_edits(edits))
    }

    /// Extract the array receiver text from a `receiver.length` expression.
    fn array_receiver_text_from_length(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<String> {
        let expression_span = self.ctx.get_span(expression_id);
        let expression_text = self.ctx.get_span_text(expression_span).trim();

        if let Some(stripped) = expression_text.strip_suffix(".length") {
            return Some(stripped.trim().to_string());
        }

        let last_dot_index = expression_text.rfind('.')?;
        let suffix = expression_text[last_dot_index + 1..].trim();
        if suffix != "length" {
            return None;
        }

        let receiver_text = expression_text[..last_dot_index].trim().to_string();
        if receiver_text.is_empty() {
            return None;
        }

        Some(receiver_text)
    }
}

/// Captured information for one `array.filter(...).length` side of a comparison.
struct FilterLengthMatch {
    /// The path for the filter receiver.
    receiver_path: ReferencePath,
    /// The arguments passed to filter.
    arguments: Vec<dir::LocalNodeId<dir::Argument>>,
    /// Whether filter has explicit generic arguments.
    has_generic_arguments: bool,
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

/// Return true when this comparison direction implies all items matched.
fn operator_implies_all_match(operator: dir::BinaryOperator, filter_on_left: bool) -> bool {
    if matches!(
        operator,
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
    ) {
        return true;
    }

    if filter_on_left && operator == dir::BinaryOperator::GreaterThanOrEqual {
        return true;
    }

    !filter_on_left && operator == dir::BinaryOperator::LessThanOrEqual
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report filter length comparisons against array length.
    #[test]
    fn test_flags_filter_length_equal_length() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_flags_filter_length_equal_length.ds",
            r#"
let items = [1, 2, 3];
let all = items.filter(item => item > 1).length === items.length;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-every")
            .assert_has_fix("prefer-array-every");
    }

    /// Report reversed filter length comparisons.
    #[test]
    fn test_flags_reversed_filter_length() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_flags_reversed_filter_length.ds",
            r#"
let items = [1, 2, 3];
let all = items.length === items.filter(item => item > 1).length;
"#,
        );
        test.result(result).assert_lint("prefer-array-every");
    }

    /// Allow comparisons against unrelated lengths.
    #[test]
    fn test_allows_mismatched_arrays() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_allows_mismatched_arrays.ds",
            r#"
let items = [1, 2, 3];
let other = [1, 2, 3];
let all = items.filter(item => item > 1).length === other.length;
"#,
        );
        test.result(result).assert_no_lint("prefer-array-every");
    }

    /// Unsafely rewrite filter-length comparisons to every calls.
    #[test]
    fn test_fix_filter_length_equal_length() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_fix_filter_length_equal_length.ds",
            r#"
let items = [1, 2, 3];
let all = items.filter(item => item > 1).length === items.length;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-every")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
let all = items.every((item) => item > 1);
"#,
            );
    }

    /// Unsafely rewrite reversed filter-length comparisons.
    #[test]
    fn test_fix_reversed_filter_length() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_fix_reversed_filter_length.ds",
            r#"
let items = [1, 2, 3];
let all = items.length === items.filter(item => item > 1).length;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-every")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
let all = items.every((item) => item > 1);
"#,
            );
    }

    /// Preserve explicit this-arg filter parameters in the rewrite.
    #[test]
    fn test_fix_preserves_filter_this_arg() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_fix_preserves_filter_this_arg.ds",
            r#"
let values = [1, 2, 3];
let context = { min: 1 };
let all = values.filter(function(value) {
    return value > this.min;
}, context).length === values.length;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-every")
            .assert_unsafe_fixed(
                r#"
let values = [1, 2, 3];
let context = { min: 1 };
let all = values.every(
    function (value) {
        return value > this.min;
    },
    context,
);
"#,
            );
    }

    /// Mutation: detect strict equality variant of filter-length check.
    #[test]
    fn test_mutation_flags_strict_equality_filter_length() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_mutation_flags_strict_equality_filter_length.ds",
            r#"
let items = [1, 2, 3];
let all = items.filter(item => item > 1).length == items.length;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-every")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
let all = items.every((item) => item > 1);
"#,
            );
    }

    /// Detect parenthesized filter-length comparisons.
    #[test]
    fn test_flags_parenthesized_filter_length_comparison() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_flags_parenthesized_filter_length_comparison.ds",
            r#"
let items = [1, 2, 3];
let all = (items.filter(item => item > 1).length) === (items.length);
"#,
        );
        test.result(result).assert_lint("prefer-array-every");
    }

    /// Detect filter-length greater-than-or-equal checks.
    #[test]
    fn test_flags_filter_length_greater_equal_length() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_flags_filter_length_greater_equal_length.ds",
            r#"
let items = [1, 2, 3];
let all = items.filter(item => item > 1).length >= items.length;
"#,
        );
        test.result(result).assert_lint("prefer-array-every");
    }

    /// Detect reversed less-than-or-equal filter-length checks.
    #[test]
    fn test_flags_length_less_equal_filter_length() {
        let test = TestProgram::for_rule_without_prelude(PreferArrayEvery);
        let result = test.lint_dir(
            "prefer_array_every/test_flags_length_less_equal_filter_length.ds",
            r#"
let items = [1, 2, 3];
let all = items.length <= items.filter(item => item > 1).length;
"#,
        );
        test.result(result).assert_lint("prefer-array-every");
    }
}
