use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    const_i64, expression_regex_literal, expression_unwrap_parenthesized, flip_binary_operator,
    is_string_type, regex_prefix_literal, single_quoted_string_literal, strip_dot_member_suffix,
};
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

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
        fixable = Sometimes,
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
    /// The string id for the startsWith method name.
    starts_with_name: StringId,
    /// The string id for the test method name.
    test_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferStringStartsWithVisitor<'a, 'b> {
    /// Build a visitor for prefer-string-startswith checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let string_symbol = ctx.well_known_symbol(WellKnownSymbol::String);
        let index_of_name = ctx.string_id("indexOf");
        let starts_with_name = ctx.string_id("startsWith");
        let test_name = ctx.string_id("test");

        Self {
            ctx,
            meta,
            string_symbol,
            index_of_name,
            starts_with_name,
            test_name,
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
        if let Some(starts_with_match) = self.match_comparison(left, operator, right, false) {
            self.report_match(expression_id, starts_with_match);
            return;
        }

        // match comparisons with the constant on the left
        if let Some(starts_with_match) = self.match_comparison(right, operator, left, true) {
            self.report_match(expression_id, starts_with_match);
        }
    }

    /// Match a comparison with a candidate expression and constant.
    fn match_comparison(
        &mut self,
        candidate_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        constant_id: dir::LocalNodeId<dir::Expression>,
        flipped: bool,
    ) -> Option<StartsWithMatch> {
        let candidate_id = expression_unwrap_parenthesized(self.ctx.tree, candidate_id);
        let constant_id = expression_unwrap_parenthesized(self.ctx.tree, constant_id);

        // resolve constant comparisons
        let constant_value = self.ctx.const_value(constant_id)?;
        let constant = const_i64(&constant_value)?;

        // normalize operators when constants are on the left
        let operator = if flipped {
            flip_binary_operator(operator)?
        } else {
            operator
        };

        // only match strict equality checks against zero
        if !matches!(
            operator,
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict
        ) {
            return None;
        }
        if constant != 0 {
            return None;
        }

        // require indexOf call forms we can safely rewrite
        let candidate = self.index_of_call_candidate(candidate_id)?;
        if let Some(from_index_id) = candidate.from_index_id
            && !self.is_zero_constant(from_index_id)
        {
            return None;
        }

        Some(StartsWithMatch {
            call_member_id: candidate.call_member_id,
            prefix_id: candidate.prefix_id,
        })
    }

    /// Report a prefer-string-startswith match.
    fn report_match(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        starts_with_match: StartsWithMatch,
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build diagnostic and attach fix
        let span = self.ctx.get_span(expression_id);
        let diagnostic = LintReport::new(
            PREFER_STRING_STARTS_WITH.id,
            PREFER_STRING_STARTS_WITH.code,
            PREFER_STRING_STARTS_WITH.category,
            severity,
            "prefer startsWith() over indexOf() === 0",
            span,
        )
        .label("use startsWith() to check the prefix");

        let mut diagnostic = diagnostic;
        if self.ctx.include_fixes
            && let Some(fix) = self.starts_with_fix(expression_id, starts_with_match)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Check one call expression for anchored regex test patterns.
    fn check_regex_test(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // match call expression
        let dir::Expression::Call {
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
        let member_expression = self.ctx.tree.get(*left);
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

        // require one static regex prefix pattern
        let Some(prefix_text) = self.regex_prefix_text(*regex_expression_id) else {
            return;
        };

        // require one positional string argument
        let first_argument = self.ctx.tree.get(arguments[0]);
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
            PREFER_STRING_STARTS_WITH.id,
            PREFER_STRING_STARTS_WITH.code,
            PREFER_STRING_STARTS_WITH.category,
            severity,
            "prefer startsWith() over regex test() prefix checks",
            span,
        )
        .label("use startsWith() for anchored prefix checks");
        if self.ctx.include_fixes
            && let Some(fix) = self.regex_starts_with_fix(expression_id, *argument_id, &prefix_text)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Return one normalized startsWith candidate from an indexOf call.
    fn index_of_call_candidate(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<StartsWithCandidate> {
        // match call expression
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
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
        if arguments.is_empty() || arguments.len() > 2 {
            return None;
        }

        // match member access for indexOf
        let call_member_id = *left;
        let member_expression = self.ctx.tree.get(call_member_id);
        let dir::Expression::Member {
            left: receiver_id,
            name,
        } = member_expression
        else {
            return None;
        };
        if *name != Some(self.index_of_name) {
            return None;
        }

        // require one positional search argument
        let first_argument = self.ctx.tree.get(arguments[0]);
        let dir::Argument::Positional {
            value: prefix_id, ..
        } = first_argument
        else {
            return None;
        };

        // optionally accept one positional fromIndex argument
        let from_index_id = if arguments.len() == 2 {
            let second_argument = self.ctx.tree.get(arguments[1]);
            let dir::Argument::Positional { value, .. } = second_argument else {
                return None;
            };
            Some(*value)
        } else {
            None
        };

        // require a string receiver
        if !self.is_string_receiver(*receiver_id) {
            return None;
        }

        Some(StartsWithCandidate {
            call_member_id,
            prefix_id: *prefix_id,
            from_index_id,
        })
    }

    /// Build a safe fix from an indexOf comparison to startsWith.
    fn starts_with_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        starts_with_match: StartsWithMatch,
    ) -> Option<LintFix> {
        // preserve receiver and prefix source text
        let member_span = self.ctx.get_span(starts_with_match.call_member_id);
        let member_text = self.ctx.get_span_text(member_span);
        let receiver_text = strip_dot_member_suffix(member_text, "indexOf")?;
        let prefix_span = self.ctx.get_span(starts_with_match.prefix_id);
        let prefix_text = self.ctx.get_span_text(prefix_span);
        let method_name = self.ctx.strings.get(self.starts_with_name);
        let replacement = format!("{receiver_text}.{}({prefix_text})", method_name);

        // replace the full comparison expression
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace indexOf() === 0 with startsWith()").with_edits(edits))
    }

    /// Return true when an expression resolves to the constant value zero.
    fn is_zero_constant(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let Some(constant_value) = self.ctx.const_value(expression_id) else {
            return false;
        };
        const_i64(&constant_value) == Some(0)
    }

    /// Return true when the receiver expression is a string type.
    fn is_string_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_string_type(self.ctx.types, type_id, Some(self.string_symbol))
    }

    /// Return one simple prefix string from a regex literal expression.
    fn regex_prefix_text(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<String> {
        let (pattern_id, flags_id) = expression_regex_literal(self.ctx.tree, expression_id)?;

        let pattern = self.ctx.strings.get(pattern_id);
        let flags = flags_id
            .map(|flags_id| self.ctx.strings.get(flags_id).to_string())
            .unwrap_or_default();
        regex_prefix_literal(pattern.as_ref(), &flags)
    }

    /// Build a safe fix from one regex test prefix check to startsWith.
    fn regex_starts_with_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
        prefix_text: &str,
    ) -> Option<LintFix> {
        let argument_span = self.ctx.get_span(argument_id);
        let argument_text = self.ctx.get_span_text(argument_span);
        if argument_text.trim().is_empty() {
            return None;
        }

        let quoted_prefix = single_quoted_string_literal(prefix_text);
        let method_name = self.ctx.strings.get(self.starts_with_name);
        let replacement = format!("({argument_text}).{}({quoted_prefix})", method_name);

        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();
        Some(LintFix::safe("Replace regex test() prefix check with startsWith()").with_edits(edits))
    }
}

impl NodeVisitor for PreferStringStartsWithVisitor<'_, '_> {
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

        // check anchored regex test prefix patterns
        self.check_regex_test(id, expression);

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// One normalized indexOf candidate for startsWith checks.
#[derive(Clone, Copy)]
struct StartsWithCandidate {
    /// The member expression for the indexOf call.
    call_member_id: dir::LocalNodeId<dir::Expression>,
    /// The searched prefix expression.
    prefix_id: dir::LocalNodeId<dir::Expression>,
    /// Optional fromIndex expression.
    from_index_id: Option<dir::LocalNodeId<dir::Expression>>,
}

/// One normalized startsWith match payload.
#[derive(Clone, Copy)]
struct StartsWithMatch {
    /// The member expression for the indexOf call.
    call_member_id: dir::LocalNodeId<dir::Expression>,
    /// The searched prefix expression.
    prefix_id: dir::LocalNodeId<dir::Expression>,
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

    /// Safely rewrite indexOf prefix checks.
    #[test]
    fn test_fix_index_of_zero_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_fix_index_of_zero_check.ds",
            r#"
let text = "hello";
let has = text.indexOf("he") === 0;
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-startswith")
            .assert_has_fix("prefer-string-startswith")
            .assert_safe_fixed(
                r#"
let text = "hello";
let has = text.startsWith("he");
"#,
            );
    }

    /// Safely rewrite flipped indexOf prefix checks.
    #[test]
    fn test_fix_flipped_index_of_zero_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_fix_flipped_index_of_zero_check.ds",
            r#"
let text = "hello";
let has = 0 === text.indexOf("he");
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-startswith")
            .assert_has_fix("prefer-string-startswith")
            .assert_safe_fixed(
                r#"
let text = "hello";
let has = text.startsWith("he");
"#,
            );
    }

    /// Allow indexOf checks with non zero fromIndex.
    #[test]
    fn test_allows_index_of_non_zero_from_index() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_allows_index_of_non_zero_from_index.ds",
            r#"
let text = "hello";
let has = text.indexOf("he", 1) === 0;
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-string-startswith");
    }

    /// Report and fix indexOf checks with explicit zero fromIndex.
    #[test]
    fn test_fix_index_of_zero_from_index() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_fix_index_of_zero_from_index.ds",
            r#"
let text = "hello";
let has = text.indexOf("he", 0) === 0;
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-startswith")
            .assert_has_fix("prefer-string-startswith")
            .assert_safe_fixed(
                r#"
let text = "hello";
let has = text.startsWith("he");
"#,
            );
    }

    /// Report parenthesized indexOf prefix checks.
    #[test]
    fn test_flags_parenthesized_index_of_zero_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_flags_parenthesized_index_of_zero_check.ds",
            r#"
let text = "hello";
let has = (text.indexOf("he")) === (0);
"#,
        );
        test.result(result).assert_lint("prefer-string-startswith");
    }

    /// Report anchored regex test prefix checks.
    #[test]
    fn test_flags_regex_test_prefix_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_flags_regex_test_prefix_check.ds",
            r#"
let text = "hello";
let has = /^he/.test(text);
"#,
        );
        test.result(result).assert_lint("prefer-string-startswith");
    }

    /// Safely rewrite anchored regex prefix tests to startsWith.
    #[test]
    fn test_fix_regex_test_prefix_check() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_fix_regex_test_prefix_check.ds",
            r#"
let text = "hello";
let has = /^he/.test(text);
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-startswith")
            .assert_has_fix("prefer-string-startswith")
            .assert_safe_fixed(
                r#"
let text = "hello";
let has = (text).startsWith('he');
"#,
            );
    }

    /// Allow regex test prefix checks with case insensitive flag.
    #[test]
    fn test_allows_regex_test_prefix_check_with_case_insensitive_flag() {
        let test = TestProgram::for_rule_without_prelude(PreferStringStartsWith);
        let result = test.lint_dir(
            "prefer_string_startswith/test_allows_regex_test_prefix_check_with_case_insensitive_flag.ds",
            r#"
let text = "hello";
let has = /^HE/i.test(text);
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-string-startswith");
    }
}
