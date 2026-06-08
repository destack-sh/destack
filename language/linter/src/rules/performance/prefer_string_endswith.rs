use destack_core::StringId;
use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_repository::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::{
    const_i64, expression_regex_literal, expression_target_symbol, expression_unwrap_parenthesized,
    is_string_type, regex_suffix_literal, single_quoted_string_literal,
    string_literal_utf16_length, strip_dot_member_suffix,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer `endsWith()` over `slice(-n) === suffix`.
    ///
    /// `endsWith()` communicates intent and avoids manual slicing.
    #[lint(
        id = "prefer-string-endswith",
        code = "LP016",
        category = Performance,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::String)],
        requires_any = [],
        fixable = Sometimes,
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
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferStringEndsWithVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags prefer-string-endswith patterns.
struct PreferStringEndsWithVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// The string id for the slice method name.
    slice_name: StringId,
    /// The string id for the endsWith method name.
    ends_with_name: StringId,
    /// The string id for the length property name.
    length_name: StringId,
    /// The string id for the test method name.
    test_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferStringEndsWithVisitor<'a, 'b> {
    /// Build a visitor for prefer-string-endswith checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        // resolve the language item String symbol for this module
        let string_symbol = ctx.language_item(LanguageItem::String);

        // intern commonly used names
        let slice_name = ctx.string_id("slice");
        let ends_with_name = ctx.string_id("endsWith");
        let length_name = ctx.string_id("length");
        let test_name = ctx.string_id("test");

        // prepare visitor state
        Self {
            ctx,
            meta,
            string_symbol,
            slice_name,
            ends_with_name,
            length_name,
            test_name,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

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
        if let Some(ends_with_match) = self.match_slice_comparison(left, right) {
            self.report_match(expression_id, ends_with_match);
            return;
        }

        // match slice calls on the right
        if let Some(ends_with_match) = self.match_slice_comparison(right, left) {
            self.report_match(expression_id, ends_with_match);
        }
    }

    /// Match a comparison with a slice call and suffix.
    fn match_slice_comparison(
        &mut self,
        slice_id: dir::LocalNodeId<dir::Expression>,
        suffix_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<EndsWithMatch> {
        // match call expression
        let expression = self.ctx.dir.get(slice_id);
        let dir::Expression::Call {
            position: _,
            left,
            generic_arguments,
            arguments,
        } = expression
        else {
            return None;
        };
        if !generic_arguments.is_empty() {
            return None;
        }
        let [argument_id] = arguments.as_slice() else {
            return None;
        };
        let argument = self.ctx.dir.get(*argument_id);
        let dir::Argument::Positional {
            value: argument_expression_id,
            ..
        } = argument
        else {
            return None;
        };

        // match member access for slice
        let member_expression = self.ctx.dir.get(*left);
        let dir::Expression::Member {
            left: receiver_id,
            name,
            ..
        } = member_expression
        else {
            return None;
        };
        if *name != Some(self.slice_name) {
            return None;
        }

        // ensure the receiver is a string
        if !self.is_string_receiver(*receiver_id) {
            return None;
        }

        // ensure the slice argument matches the suffix length
        if !self.is_suffix_length_match(*receiver_id, *argument_expression_id, suffix_id) {
            return None;
        }

        Some(EndsWithMatch {
            call_member_id: *left,
            suffix_id,
        })
    }

    /// Report a prefer-string-endswith match.
    fn report_match(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        ends_with_match: EndsWithMatch,
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build diagnostic and attach fix
        let span = self.ctx.get_span(expression_id);
        let diagnostic = LintReport::new(
            PREFER_STRING_ENDS_WITH.id,
            PREFER_STRING_ENDS_WITH.code,
            PREFER_STRING_ENDS_WITH.category,
            severity,
            "prefer endsWith() over slice(-n) comparison",
            span,
        )
        .label("use endsWith() to check the suffix");

        let mut diagnostic = diagnostic;
        if self.ctx.compute_fixes
            && let Some(fix) = self.ends_with_fix(expression_id, ends_with_match)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Check one call expression for anchored regex test suffix patterns.
    fn check_regex_test(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // match call expression
        let dir::Expression::Call {
            position: _,
            left,
            generic_arguments,
            arguments,
        } = expression
        else {
            return;
        };
        if !generic_arguments.is_empty() {
            return;
        }
        if arguments.len() != 1 {
            return;
        }

        // match `.test(...)` call
        let member_expression = self.ctx.dir.get(*left);
        let dir::Expression::Member {
            left: regex_expression_id,
            name,
        } = member_expression
        else {
            return;
        };
        if *name != Some(self.test_name) {
            return;
        }
        if !generic_arguments.is_empty() {
            return;
        }

        // require one static regex suffix pattern
        let Some(suffix_text) = self.regex_suffix_text(*regex_expression_id) else {
            return;
        };

        // require one positional string argument
        let first_argument = self.ctx.dir.get(arguments[0]);
        let dir::Argument::Positional {
            value: argument_id, ..
        } = first_argument
        else {
            return;
        };
        if !self.is_string_receiver(*argument_id) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build diagnostic and attach safe fix
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            PREFER_STRING_ENDS_WITH.id,
            PREFER_STRING_ENDS_WITH.code,
            PREFER_STRING_ENDS_WITH.category,
            severity,
            "prefer endsWith() over regex test() suffix checks",
            span,
        )
        .label("use endsWith() for anchored suffix checks");
        if self.ctx.compute_fixes
            && let Some(fix) = self.regex_ends_with_fix(expression_id, *argument_id, &suffix_text)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return true when the receiver expression is a string type.
    fn is_string_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_string_type(self.ctx, type_id, Some(self.string_symbol))
    }

    /// Return true when the slice argument matches the suffix length.
    fn is_suffix_length_match(
        &mut self,
        receiver_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
        suffix_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        // check for `-suffix.length` patterns
        if self.is_suffix_length_argument(argument_id, suffix_id) {
            return true;
        }

        // check for `receiver.length - suffix.length` patterns
        if self.is_receiver_length_minus_suffix_length_argument(receiver_id, argument_id, suffix_id)
        {
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

    /// Build a safe fix from one slice suffix comparison to endsWith.
    fn ends_with_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        ends_with_match: EndsWithMatch,
    ) -> Option<LintFix> {
        // preserve receiver and suffix source text
        let member_span = self.ctx.get_span(ends_with_match.call_member_id);
        let member_text = self.ctx.get_span_text(member_span);
        let receiver_text = strip_dot_member_suffix(member_text, "slice")?;
        let suffix_span = self.ctx.get_span(ends_with_match.suffix_id);
        let suffix_text = self.ctx.get_span_text(suffix_span);
        let method_name = self.ctx.strings.get(self.ends_with_name);
        let replacement = format!("{receiver_text}.{method_name}({suffix_text})");

        // replace the full comparison expression
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace slice() suffix check with endsWith()").with_edits(edits))
    }

    /// Return one simple suffix string from a regex literal expression.
    fn regex_suffix_text(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<String> {
        let (pattern_id, flags_id) = expression_regex_literal(self.ctx.dir.tree(), expression_id)?;

        let pattern = self.ctx.strings.get(pattern_id);
        let flags = flags_id
            .map(|flags_id| self.ctx.strings.get(flags_id).to_string())
            .unwrap_or_default();
        regex_suffix_literal(pattern, &flags)
    }

    /// Build a safe fix from one regex test suffix check to endsWith.
    fn regex_ends_with_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
        suffix_text: &str,
    ) -> Option<LintFix> {
        let argument_span = self.ctx.get_span(argument_id);
        let argument_text = self.ctx.get_span_text(argument_span);
        if argument_text.trim().is_empty() {
            return None;
        }

        let quoted_suffix = single_quoted_string_literal(suffix_text);
        let method_name = self.ctx.strings.get(self.ends_with_name);
        let replacement = format!("({argument_text}).{method_name}({quoted_suffix})");

        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();
        Some(LintFix::safe("Replace regex test() suffix check with endsWith()").with_edits(edits))
    }

    /// Return true when the argument is `-suffix.length`.
    fn is_suffix_length_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Expression>,
        suffix_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        // unwrap negation
        let argument_expression = self.ctx.dir.get(argument_id);
        let dir::Expression::Unary {
            operator: dir::UnaryOperator::Negate,
            right,
        } = argument_expression
        else {
            return false;
        };

        // match suffix.length member access
        let member_expression = self.ctx.dir.get(*right);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return false;
        };
        if *name != Some(self.length_name) {
            return false;
        }

        // compare suffix symbols when possible
        let Some(expected_symbol) = expression_target_symbol(self.ctx, *left) else {
            return false;
        };
        let Some(actual_symbol) = expression_target_symbol(self.ctx, suffix_id) else {
            return false;
        };

        expected_symbol == actual_symbol
    }

    /// Return the length for a string literal suffix.
    fn string_literal_length(&self, suffix_id: dir::LocalNodeId<dir::Expression>) -> Option<usize> {
        // unwrap parenthesized expressions
        let suffix_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), suffix_id);

        // match string literals
        let expression = self.ctx.dir.get(suffix_id);
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(value)) = expression else {
            return None;
        };

        Some(string_literal_utf16_length(self.ctx.strings, *value))
    }

    /// Return true when the argument is `receiver.length - suffix.length`.
    fn is_receiver_length_minus_suffix_length_argument(
        &mut self,
        receiver_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
        suffix_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let argument_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), argument_id);
        let argument_expression = self.ctx.dir.get(argument_id);
        let dir::Expression::Binary {
            left,
            operator: dir::BinaryOperator::Subtract,
            right,
        } = argument_expression
        else {
            return false;
        };

        self.is_length_member_of(*left, receiver_id) && self.is_length_member_of(*right, suffix_id)
    }

    /// Return true when one expression is `<target>.length`.
    fn is_length_member_of(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
        let target_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), target_id);

        let expression = self.ctx.dir.get(expression_id);
        let dir::Expression::Member { left, name, .. } = expression else {
            return false;
        };
        if *name != Some(self.length_name) {
            return false;
        }

        if let (Some(expected_symbol), Some(actual_symbol)) = (
            expression_target_symbol(self.ctx, *left),
            expression_target_symbol(self.ctx, target_id),
        ) {
            return expected_symbol == actual_symbol;
        }

        let left_span = self.ctx.get_span(*left);
        let target_span = self.ctx.get_span(target_id);
        let left_text = self.ctx.get_span_text(left_span);
        let target_text = self.ctx.get_span_text(target_span);
        left_text.trim() == target_text.trim()
    }
}

impl NodeVisitor for PreferStringEndsWithVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
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

        // check anchored regex test suffix patterns
        self.check_regex_test(id, expression);

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// One normalized endsWith match payload.
#[derive(Clone, Copy)]
struct EndsWithMatch {
    /// The member expression for the slice call.
    call_member_id: dir::LocalNodeId<dir::Expression>,
    /// The suffix expression.
    suffix_id: dir::LocalNodeId<dir::Expression>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report slice comparisons against suffix lengths.
    #[test]
    fn test_flags_slice_suffix_length_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_flags_slice_suffix_length_check.ds",
            r#"
let text = "hello";
let suffix = "lo";
let ends = text.slice(-suffix.length) === suffix;
"#,
        );
        test.result(result).assert_lint("prefer-string-endswith");
    }

    /// Report literal slice comparisons.
    #[test]
    fn test_flags_literal_suffix_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_flags_literal_suffix_check.ds",
            r#"
let text = "hello";
let ends = text.slice(-2) === "lo";
"#,
        );
        test.result(result).assert_lint("prefer-string-endswith");
    }

    /// Allow unrelated slice comparisons.
    #[test]
    fn test_allows_unrelated_slice_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_allows_unrelated_slice_check.ds",
            r#"
let text = "hello";
let ends = text.slice(0) === "hello";
"#,
        );
        test.result(result).assert_no_lint("prefer-string-endswith");
    }

    /// Safely rewrite slice suffix comparisons.
    #[test]
    fn test_fix_slice_suffix_length_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_fix_slice_suffix_length_check.ds",
            r#"
let text = "hello";
let suffix = "lo";
let ends = text.slice(-suffix.length) === suffix;
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-endswith")
            .assert_has_fix("prefer-string-endswith")
            .assert_safe_fixed(
                r#"
let text = "hello";
let suffix = "lo";
let ends = text.endsWith(suffix);
"#,
            );
    }

    /// Safely rewrite literal suffix slice comparisons.
    #[test]
    fn test_fix_literal_suffix_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_fix_literal_suffix_check.ds",
            r#"
let text = "hello";
let ends = text.slice(-2) === "lo";
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-endswith")
            .assert_has_fix("prefer-string-endswith")
            .assert_safe_fixed(
                r#"
let text = "hello";
let ends = text.endsWith("lo");
"#,
            );
    }

    /// Safely rewrite flipped slice suffix comparisons.
    #[test]
    fn test_fix_flipped_slice_suffix_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_fix_flipped_slice_suffix_check.ds",
            r#"
let text = "hello";
let suffix = "lo";
let ends = suffix === text.slice(-suffix.length);
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-endswith")
            .assert_has_fix("prefer-string-endswith")
            .assert_safe_fixed(
                r#"
let text = "hello";
let suffix = "lo";
let ends = text.endsWith(suffix);
"#,
            );
    }

    /// Report receiver length delta slice comparisons.
    #[test]
    fn test_flags_receiver_length_delta_suffix_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_flags_receiver_length_delta_suffix_check.ds",
            r#"
let text = "hello";
let suffix = "lo";
let ends = text.slice(text.length - suffix.length) === suffix;
"#,
        );
        test.result(result).assert_lint("prefer-string-endswith");
    }

    /// Report anchored regex test suffix checks.
    #[test]
    fn test_flags_regex_test_suffix_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_flags_regex_test_suffix_check.ds",
            r#"
let text = "hello";
let has = /lo$/.test(text);
"#,
        );
        test.result(result).assert_lint("prefer-string-endswith");
    }

    /// Safely rewrite anchored regex suffix tests to endsWith.
    #[test]
    fn test_fix_regex_test_suffix_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_fix_regex_test_suffix_check.ds",
            r#"
let text = "hello";
let has = /lo$/.test(text);
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-endswith")
            .assert_has_fix("prefer-string-endswith")
            .assert_safe_fixed(
                r#"
let text = "hello";
let has = (text).endsWith('lo');
"#,
            );
    }

    /// Allow regex test suffix checks with multiline flag.
    #[test]
    fn test_allows_regex_test_suffix_check_with_multiline_flag() {
        let test = TestProgram::for_rule_without_prelude(PreferStringEndsWith);
        let result = test.lint_dir(
            "prefer_string_endswith/test_allows_regex_test_suffix_check_with_multiline_flag.ds",
            r#"
let text = "hello";
let has = /lo$/m.test(text);
"#,
        );
        test.result(result).assert_no_lint("prefer-string-endswith");
    }
}
