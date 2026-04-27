use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{
    const_i64, expression_regex_literal, expression_unwrap_parenthesized, flip_binary_operator,
    is_array_type, is_string_type, single_quoted_string_literal, strip_dot_member_suffix,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `includes()` over `indexOf()` comparisons and simple regex tests.
    ///
    /// `includes()` communicates intent more clearly and avoids comparisons
    /// against sentinel values.
    #[lint(
        id = "prefer-includes",
        code = "LP015",
        category = Performance,
        level = Dir,
        requires_all = [
            RequireWellKnownSymbol(WellKnownSymbol::Array),
            RequireWellKnownSymbol(WellKnownSymbol::String),
        ],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferIncludes,
    "Prefer includes() over indexOf() comparisons and simple regex test() calls"
}

impl LintRule for PreferIncludes {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferIncludes::meta()
    }

    /// Check module DIR nodes for indexOf comparisons that should use includes().
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferIncludesVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// The kind of indexOf comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncludesCheck {
    /// The comparison checks that a match exists.
    AnyMatch,
    /// The comparison checks that no match exists.
    NoMatch,
}

/// The matched index method kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncludesMethod {
    /// The `indexOf` method.
    IndexOf,
    /// The `lastIndexOf` method.
    LastIndexOf,
}

/// One normalized includes candidate call.
#[derive(Debug, Clone, Copy, PartialEq)]
struct IncludesCandidate {
    /// The method kind.
    method: IncludesMethod,
    /// The member expression id.
    call_member_id: dir::LocalNodeId<dir::Expression>,
    /// The searched value expression id.
    search_id: dir::LocalNodeId<dir::Expression>,
    /// Optional from-index expression id.
    from_index_id: Option<dir::LocalNodeId<dir::Expression>>,
}

/// One normalized includes match payload.
#[derive(Debug, Clone, Copy, PartialEq)]
struct IncludesMatch {
    /// The comparison intent.
    check: IncludesCheck,
    /// The matched includes candidate.
    candidate: IncludesCandidate,
}

/// Node visitor that flags prefer-includes patterns.
struct PreferIncludesVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The well known String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// The string id for the indexOf method name.
    index_of_name: StringId,
    /// The string id for the lastIndexOf method name.
    last_index_of_name: StringId,
    /// The string id for the includes method name.
    includes_name: StringId,
    /// The string id for the test method name.
    test_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferIncludesVisitor<'a, 'b> {
    /// Build a visitor for prefer-includes checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let string_symbol = ctx.well_known_symbol(WellKnownSymbol::String);
        let index_of_name = ctx.repository.strings.intern("indexOf");
        let last_index_of_name = ctx.repository.strings.intern("lastIndexOf");
        let includes_name = ctx.repository.strings.intern("includes");
        let test_name = ctx.repository.strings.intern("test");

        Self {
            ctx,
            meta,
            array_symbol,
            string_symbol,
            index_of_name,
            last_index_of_name,
            includes_name,
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

    /// Check comparison expressions for prefer-includes patterns.
    fn check_binary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        // match comparisons with the constant on the right
        if let Some(includes_match) = self.match_comparison(left, operator, right, false) {
            self.report_match(expression_id, includes_match);
            return;
        }

        // match comparisons with the constant on the left
        if let Some(includes_match) = self.match_comparison(right, operator, left, true) {
            self.report_match(expression_id, includes_match);
        }
    }

    /// Match a comparison with a candidate expression and constant.
    fn match_comparison(
        &mut self,
        candidate_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        constant_id: dir::LocalNodeId<dir::Expression>,
        flipped: bool,
    ) -> Option<IncludesMatch> {
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

        // check for indexOf and lastIndexOf comparisons
        let candidate = self.index_call_candidate(candidate_id)?;
        let check = check_index_of_comparison(operator, constant)?;

        // only allow from-index forms that stay equivalent to includes
        if !self.is_includes_equivalent(candidate) {
            return None;
        }

        Some(IncludesMatch { check, candidate })
    }

    /// Report a prefer-includes match.
    fn report_match(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        includes_match: IncludesMatch,
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build diagnostic text
        let label = match includes_match.check {
            IncludesCheck::AnyMatch => "use includes() to check for a match",
            IncludesCheck::NoMatch => "use !includes() to check for no matches",
        };

        // build diagnostic and attach fix when safe
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_INCLUDES.id,
            PREFER_INCLUDES.code,
            PREFER_INCLUDES.category,
            severity,
            "prefer includes() over indexOf() comparison",
            self.ctx.module.file_id,
            span,
        )
        .with_label(label);
        if self.ctx.include_fixes
            && let Some(fix) = self.includes_fix(expression_id, includes_match)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Check one call expression for simple regex-test patterns.
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
        if !generic_arguments.is_empty() {
            return;
        }

        // match a plain regex literal pattern
        let Some(pattern_text) = self.regex_plain_pattern_text(*regex_expression_id) else {
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
        let Some(argument_type_id) = self.ctx.expression_type_id(*argument_id) else {
            return;
        };
        if !is_string_type(self.ctx.types, argument_type_id, Some(self.string_symbol)) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build the diagnostic and optional fix
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_INCLUDES.id,
            PREFER_INCLUDES.code,
            PREFER_INCLUDES.category,
            severity,
            "prefer includes() over simple regex test()",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use includes() for simple substring checks");
        if self.ctx.include_fixes
            && let Some(fix) = self.regex_test_fix(expression_id, *argument_id, &pattern_text)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Parse one normalized index method call candidate.
    fn index_call_candidate(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<IncludesCandidate> {
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

        // match member access for indexOf or lastIndexOf
        let call_member_id = *left;
        let member_expression = self.ctx.tree.get(call_member_id);
        let dir::Expression::Member {
            left: receiver_id,
            name,
        } = member_expression
        else {
            return None;
        };
        let method = if *name == Some(self.index_of_name) {
            IncludesMethod::IndexOf
        } else if *name == Some(self.last_index_of_name) {
            IncludesMethod::LastIndexOf
        } else {
            return None;
        };

        // require positional search argument
        let first_argument = self.ctx.tree.get(arguments[0]);
        let dir::Argument::Positional {
            value: search_id, ..
        } = first_argument
        else {
            return None;
        };

        // optionally collect one positional from-index argument
        let from_index_id = if arguments.len() == 2 {
            let second_argument = self.ctx.tree.get(arguments[1]);
            let dir::Argument::Positional { value, .. } = second_argument else {
                return None;
            };
            Some(*value)
        } else {
            None
        };

        // ensure the receiver is supported
        if !self.is_supported_receiver(*receiver_id) {
            return None;
        }

        Some(IncludesCandidate {
            method,
            call_member_id,
            search_id: *search_id,
            from_index_id,
        })
    }

    /// Return true when an index method candidate is equivalent to includes.
    fn is_includes_equivalent(&mut self, candidate: IncludesCandidate) -> bool {
        match candidate.method {
            // `indexOf(search)` and `indexOf(search, 0)` are equivalent to includes
            IncludesMethod::IndexOf => match candidate.from_index_id {
                None => true,
                Some(from_index_id) => self.is_zero_constant(from_index_id),
            },
            // `lastIndexOf` is only equivalent without from-index
            IncludesMethod::LastIndexOf => candidate.from_index_id.is_none(),
        }
    }

    /// Build a safe fix from one index comparison to includes.
    fn includes_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        includes_match: IncludesMatch,
    ) -> Option<LintFix> {
        // derive receiver text from member expression text
        let member_span = self.ctx.get_span(includes_match.candidate.call_member_id);
        let member_text = self.ctx.get_span_text(member_span);
        let receiver_text = match includes_match.candidate.method {
            IncludesMethod::IndexOf => strip_dot_member_suffix(member_text, "indexOf")?,
            IncludesMethod::LastIndexOf => strip_dot_member_suffix(member_text, "lastIndexOf")?,
        };

        // preserve search argument source text
        let search_span = self.ctx.get_span(includes_match.candidate.search_id);
        let search_text = self.ctx.get_span_text(search_span);
        let includes_name = self.ctx.repository.strings.get(self.includes_name);
        let includes_call = format!("{receiver_text}.{}({search_text})", includes_name.as_ref());
        let replacement = match includes_match.check {
            IncludesCheck::AnyMatch => includes_call,
            IncludesCheck::NoMatch => format!("!{includes_call}"),
        };

        // replace the full comparison expression
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace indexOf comparison with includes()").with_edits(edits))
    }

    /// Return true when an expression resolves to the constant value zero.
    fn is_zero_constant(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let Some(constant_value) = self.ctx.const_value(expression_id) else {
            return false;
        };
        const_i64(&constant_value) == Some(0)
    }

    /// Return true when the receiver expression is an array or string type.
    fn is_supported_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        let is_array = is_array_type(self.ctx.types, type_id, Some(self.array_symbol));
        if is_array {
            return true;
        }

        is_string_type(self.ctx.types, type_id, Some(self.string_symbol))
    }

    /// Return one plain substring pattern for a simple regex literal.
    fn regex_plain_pattern_text(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<String> {
        // unwrap parenthesized wrappers around the regex expression
        let expression_id = expression_unwrap_parenthesized(self.ctx.tree, expression_id);

        // match regex scalar literals
        let (content, flags) = expression_regex_literal(self.ctx.tree, expression_id)?;

        // reject all regex flags for this conservative rewrite
        if let Some(flags_id) = flags {
            let flags_text = self.ctx.repository.strings.get(flags_id);
            if !flags_text.is_empty() {
                return None;
            }
        }

        // require a plain literal body without regex operators
        let pattern_text = self.ctx.repository.strings.get(content);
        plain_regex_substring(pattern_text.as_ref())
    }

    /// Build a safe fix from one simple regex-test call to includes.
    fn regex_test_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: dir::LocalNodeId<dir::Expression>,
        pattern_text: &str,
    ) -> Option<LintFix> {
        // preserve source text for the searched string expression
        let argument_span = self.ctx.get_span(argument_id);
        let argument_text = self.ctx.get_span_text(argument_span);
        if argument_text.trim().is_empty() {
            return None;
        }

        // build replacement text
        let quoted_pattern = single_quoted_string_literal(pattern_text);
        let replacement = format!("({argument_text}).includes({quoted_pattern})");
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace simple regex test() with includes()").with_edits(edits))
    }
}

impl NodeVisitor for PreferIncludesVisitor<'_, '_> {
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

        // check simple regex test expressions
        self.check_regex_test(id, expression);

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Check indexOf comparisons against constants.
fn check_index_of_comparison(
    operator: dir::BinaryOperator,
    constant: i64,
) -> Option<IncludesCheck> {
    match operator {
        dir::BinaryOperator::GreaterThan if constant == -1 => Some(IncludesCheck::AnyMatch),
        dir::BinaryOperator::GreaterThanOrEqual if constant == 0 => Some(IncludesCheck::AnyMatch),
        dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict if constant == -1 => {
            Some(IncludesCheck::AnyMatch)
        }
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict if constant == -1 => {
            Some(IncludesCheck::NoMatch)
        }
        dir::BinaryOperator::LessThan if constant == 0 => Some(IncludesCheck::NoMatch),
        dir::BinaryOperator::LessThanOrEqual if constant == -1 => Some(IncludesCheck::NoMatch),
        _ => None,
    }
}

/// Convert one regex source pattern to a plain substring when possible.
fn plain_regex_substring(pattern: &str) -> Option<String> {
    if pattern.is_empty() {
        return None;
    }

    let is_plain = pattern.chars().all(|character| {
        !matches!(
            character,
            '\\' | '^' | '$' | '*' | '+' | '?' | '.' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
        )
    });
    if !is_plain {
        return None;
    }

    Some(pattern.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report indexOf comparisons against -1.
    #[test]
    fn test_flags_index_of_not_found_check() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_flags_index_of_not_found_check.ds",
            r#"
let items = [1, 2, 3];
let has = items.indexOf(2) !== -1;
"#,
        );
        test.result(result).assert_lint("prefer-includes");
    }

    /// Report string indexOf comparisons against -1.
    #[test]
    fn test_flags_string_index_of_check() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_flags_string_index_of_check.ds",
            r#"
let text = "hello";
let has = text.indexOf("lo") != -1;
"#,
        );
        test.result(result).assert_lint("prefer-includes");
    }

    /// Allow indexOf comparisons that are not includes checks.
    #[test]
    fn test_allows_index_of_zero_check() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_allows_index_of_zero_check.ds",
            r#"
let items = [1, 2, 3];
let first = items.indexOf(2) === 0;
"#,
        );
        test.result(result).assert_no_lint("prefer-includes");
    }

    /// Safely rewrite indexOf any-match checks.
    #[test]
    fn test_fix_index_of_any_match() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_fix_index_of_any_match.ds",
            r#"
let items = [1, 2, 3];
let has = items.indexOf(2) !== -1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-includes")
            .assert_has_fix("prefer-includes")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let has = items.includes(2);
"#,
            );
    }

    /// Safely rewrite indexOf no-match checks.
    #[test]
    fn test_fix_index_of_no_match() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_fix_index_of_no_match.ds",
            r#"
let items = [1, 2, 3];
let missing = items.indexOf(5) === -1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-includes")
            .assert_has_fix("prefer-includes")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let missing = !items.includes(5);
"#,
            );
    }

    /// Safely rewrite lastIndexOf any-match checks without from-index.
    #[test]
    fn test_fix_last_index_of_any_match() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_fix_last_index_of_any_match.ds",
            r#"
let items = [1, 2, 3];
let has = items.lastIndexOf(2) >= 0;
"#,
        );
        test.result(result)
            .assert_lint("prefer-includes")
            .assert_has_fix("prefer-includes")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let has = items.includes(2);
"#,
            );
    }

    /// Allow indexOf checks with non-zero from-index.
    #[test]
    fn test_allows_index_of_non_zero_from_index() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_allows_index_of_non_zero_from_index.ds",
            r#"
let items = [1, 2, 3];
let has = items.indexOf(2, 1) !== -1;
"#,
        );
        test.result(result).assert_no_lint("prefer-includes");
    }

    /// Allow lastIndexOf checks with explicit from-index.
    #[test]
    fn test_allows_last_index_of_with_from_index() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_allows_last_index_of_with_from_index.ds",
            r#"
let items = [1, 2, 3];
let has = items.lastIndexOf(2, 0) !== -1;
"#,
        );
        test.result(result).assert_no_lint("prefer-includes");
    }

    /// Report parenthesized indexOf comparisons.
    #[test]
    fn test_flags_parenthesized_index_of_comparison() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_flags_parenthesized_index_of_comparison.ds",
            r#"
let items = [1, 2, 3];
let has = (items.indexOf(2)) !== (-1);
"#,
        );
        test.result(result).assert_lint("prefer-includes");
    }

    /// Report simple regex test calls on strings.
    #[test]
    fn test_flags_simple_regex_test() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_flags_simple_regex_test.ds",
            r#"
let text = "hello";
let has = /ell/.test(text);
"#,
        );
        test.result(result).assert_lint("prefer-includes");
    }

    /// Safely rewrite simple regex test calls to includes.
    #[test]
    fn test_fix_simple_regex_test() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_fix_simple_regex_test.ds",
            r#"
let text = "hello";
let has = /ell/.test(text);
"#,
        );
        test.result(result)
            .assert_lint("prefer-includes")
            .assert_safe_fixed(
                r#"
let text = "hello";
let has = (text).includes('ell');
"#,
            );
    }

    /// Allow complex regex patterns that are not plain substrings.
    #[test]
    fn test_allows_complex_regex_test() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_allows_complex_regex_test.ds",
            r#"
let text = "hello";
let has = /e+l/.test(text);
"#,
        );
        test.result(result).assert_no_lint("prefer-includes");
    }

    /// Allow regex tests that use behavior changing flags.
    #[test]
    fn test_allows_regex_test_with_case_insensitive_flag() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "prefer_includes/test_allows_regex_test_with_case_insensitive_flag.ds",
            r#"
let text = "hello";
let has = /ELL/i.test(text);
"#,
        );
        test.result(result).assert_no_lint("prefer-includes");
    }
}
