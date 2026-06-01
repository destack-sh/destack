use std::collections::HashSet;

use destack_core::StringId;
use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_source::Span;
use destack_workspace::LintSeverity;
use regex_syntax::hir::HirKind;

use crate::LintRequirement::RequireLanguageItem;
use crate::analysis::LintRegexParse;
use crate::rules::common::{
    expression_is_symbol, expression_static_string_literal, expression_target_symbol,
    expression_unwrap_parenthesized, is_string_type, single_quoted_string_literal,
    span_has_comment, symbol_initializer_expression,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer `replaceAll()` over `replace()` with a global regex.
    ///
    /// `replaceAll()` avoids regex overhead and better communicates intent.
    #[lint(
        id = "prefer-string-replaceall",
        code = "LY054",
        category = Style,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::String)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferStringReplaceAll,
    "Prefer string.replaceAll() over replace() with a global regex"
}

impl LintRule for PreferStringReplaceAll {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferStringReplaceAll::meta()
    }

    /// Check module DIR nodes for replace calls that should use replaceAll().
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferStringReplaceAllVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags prefer-string-replaceall patterns.
struct PreferStringReplaceAllVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The language item String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// The string id for the replace method name.
    replace_name: StringId,
    /// The string id for the replaceAll method name.
    replace_all_name: StringId,
    /// The RegExp constructor symbol when available.
    regexp_symbol: Option<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferStringReplaceAllVisitor<'a, 'b> {
    /// Build a visitor for prefer-string-replaceall checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let string_symbol = ctx.language_item(LanguageItem::String);
        let replace_name = ctx.string_id("replace");
        let replace_all_name = ctx.string_id("replaceAll");
        let regexp_name = ctx.string_id("RegExp");
        let regexp_symbol = ctx.get_declared_library_symbol(regexp_name);

        Self {
            ctx,
            meta,
            string_symbol,
            replace_name,
            replace_all_name,
            regexp_symbol,
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

    /// Check a replace call expression for replaceAll usage.
    fn check_replace_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        // skip static arguments until we support rendering them
        if !generic_arguments.is_empty() {
            return;
        }

        // match member access for replace methods
        let member_expression = self.ctx.dir.get(left);
        let dir::Expression::Member {
            left: receiver_id,
            name,
        } = member_expression
        else {
            return;
        };
        let is_replace = *name == Some(self.replace_name);
        let is_replace_all = *name == Some(self.replace_all_name);
        if !is_replace && !is_replace_all {
            return;
        }

        // ensure the receiver is a string
        if !self.is_string_receiver(*receiver_id) {
            return;
        }

        // require at least one argument
        let Some(first_argument_id) = arguments.first() else {
            return;
        };
        let first_argument = self.ctx.dir.get(*first_argument_id);
        let Some(first_argument_value_id) = first_argument.value() else {
            return;
        };
        if !self.is_global_regex(first_argument_value_id) {
            return;
        }
        let pattern_replacement =
            self.regex_pattern_literal_replacement(first_argument_value_id, first_argument_id);

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report replaceAll pattern suggestions
        if is_replace_all {
            let Some(pattern_replacement) = pattern_replacement else {
                return;
            };
            let first_argument_span = self.ctx.get_span(*first_argument_id);
            let mut diagnostic = LintReport::new(
                PREFER_STRING_REPLACE_ALL.id,
                PREFER_STRING_REPLACE_ALL.code,
                PREFER_STRING_REPLACE_ALL.category,
                severity,
                "regex pattern can be replaced with a string literal",
                first_argument_span,
            )
            .label("use a string literal pattern");
            if let Some(fix) = self.replace_call_fix(
                expression_id,
                left,
                is_replace,
                *first_argument_id,
                Some(pattern_replacement),
            ) {
                diagnostic = diagnostic.fix(fix);
            }

            self.ctx.report(diagnostic);
            return;
        }

        // report replace method suggestions
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            PREFER_STRING_REPLACE_ALL.id,
            PREFER_STRING_REPLACE_ALL.code,
            PREFER_STRING_REPLACE_ALL.category,
            severity,
            "prefer replaceAll() over replace() with a global regex",
            span,
        )
        .label("use replaceAll() for global replacements");
        if let Some(fix) = self.replace_call_fix(
            expression_id,
            left,
            is_replace,
            *first_argument_id,
            pattern_replacement,
        ) {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix for replace and replaceAll regex improvements.
    fn replace_call_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        member_id: dir::LocalNodeId<dir::Expression>,
        is_replace: bool,
        first_argument_id: dir::LocalNodeId<dir::Argument>,
        pattern_replacement: Option<String>,
    ) -> Option<LintFix> {
        // avoid rewriting commented calls
        let expression_span = self.ctx.get_span(expression_id);
        if span_has_comment(self.ctx.dir.tree(), expression_span) {
            return None;
        }

        // build targeted edits for method and pattern updates
        let mut edit_builder = self.ctx.edit_builder();

        if is_replace {
            let member_span = self.ctx.get_span(member_id);
            let replace_name = self.ctx.strings.get(self.replace_name);
            let method_span = Span::new(
                member_span.file,
                member_span.end.saturating_sub(replace_name.len() as u32),
                member_span.end,
            );
            let replace_all_name = self.ctx.strings.get(self.replace_all_name);
            edit_builder = edit_builder.replace(method_span, replace_all_name);
        }

        if let Some(pattern_replacement) = pattern_replacement {
            let first_argument_span = self.ctx.get_span(first_argument_id);
            edit_builder = edit_builder.replace(first_argument_span, pattern_replacement);
        }

        let edits = edit_builder.into_edits();
        if edits.is_empty() {
            return None;
        }

        Some(LintFix::safe("Use replaceAll with string pattern").with_edits(edits))
    }

    /// Return true when the receiver expression is a string type.
    fn is_string_receiver(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_string_type(self.ctx, type_id, Some(self.string_symbol))
    }

    /// Return true when the expression is a global regex literal.
    fn is_global_regex(&self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let mut visited_symbols = HashSet::new();
        self.is_global_regex_expression(expression_id, &mut visited_symbols)
    }

    /// Return true when one expression is a global regex literal after alias resolution.
    fn is_global_regex_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        visited_symbols: &mut HashSet<dir::GlobalSymbolId>,
    ) -> bool {
        // unwrap parenthesized expressions
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);

        // match regex literals
        let expression = self.ctx.dir.get(expression_id);
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::RegexString {
            flags: Some(flags),
            ..
        }) = expression
        else {
            if self.is_regexp_constructor_with_global_flag(expression_id, visited_symbols) {
                return true;
            }

            // follow direct symbol aliases for const regex bindings
            let Some(target_symbol) = expression_target_symbol(self.ctx, expression_id) else {
                return false;
            };
            if !visited_symbols.insert(target_symbol) {
                return false;
            }

            let Some(initializer_id) = symbol_initializer_expression(self.ctx, target_symbol)
            else {
                return false;
            };

            return self.is_global_regex_expression(initializer_id, visited_symbols);
        };

        // inspect regex flags
        let flags = self.ctx.strings.get(*flags);
        flags.contains('g')
    }

    /// Return true when the expression is `RegExp(..., "g")` or `new RegExp(..., "g")`.
    fn is_regexp_constructor_with_global_flag(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        visited_symbols: &mut HashSet<dir::GlobalSymbolId>,
    ) -> bool {
        let expression = self.ctx.dir.get(expression_id);
        let (callee_id, arguments) = match expression {
            dir::Expression::Call {
                left, arguments, ..
            } => (*left, arguments.as_slice()),
            _ => return false,
        };

        if !self.is_regexp_constructor_callee(callee_id) {
            return false;
        }

        // when explicit flags are present they override regex-literal flags
        if let Some(flags_argument_id) = arguments.get(1) {
            let flags_argument = self.ctx.dir.get(*flags_argument_id);
            let dir::Argument::Positional {
                value: flags_expression_id,
                ..
            } = flags_argument
            else {
                return false;
            };
            let Some(flags_id) =
                self.static_string_from_expression(*flags_expression_id, visited_symbols)
            else {
                return false;
            };
            let flags_text = self.ctx.strings.get(flags_id);
            return flags_text.contains('g');
        }

        // without explicit flags, inherit from first regex-literal argument
        let Some(pattern_argument_id) = arguments.first() else {
            return false;
        };
        let pattern_argument = self.ctx.dir.get(*pattern_argument_id);
        let dir::Argument::Positional {
            value: pattern_expression_id,
            ..
        } = pattern_argument
        else {
            return false;
        };

        self.is_global_regex_expression(*pattern_expression_id, visited_symbols)
    }

    /// Resolve one static string literal through local symbol aliases.
    fn static_string_from_expression(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        visited_symbols: &mut HashSet<dir::GlobalSymbolId>,
    ) -> Option<StringId> {
        // unwrap parenthesized wrappers
        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);

        // keep direct static string literals
        if let Some(string_id) =
            expression_static_string_literal(self.ctx.dir.tree(), expression_id)
        {
            return Some(string_id);
        }

        // follow local symbol aliases for const string flags
        let target_symbol = expression_target_symbol(self.ctx, expression_id)?;
        if !visited_symbols.insert(target_symbol) {
            return None;
        }

        let initializer_id = symbol_initializer_expression(self.ctx, target_symbol)?;

        self.static_string_from_expression(initializer_id, visited_symbols)
    }

    /// Return true when one expression resolves to the global RegExp constructor.
    fn is_regexp_constructor_callee(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let Some(regexp_symbol) = self.regexp_symbol else {
            return false;
        };

        expression_is_symbol(self.ctx, expression_id, regexp_symbol)
    }

    /// Build one string literal replacement from a simple global regex argument.
    fn regex_pattern_literal_replacement(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        argument_id: &dir::LocalNodeId<dir::Argument>,
    ) -> Option<String> {
        let argument = self.ctx.dir.get(*argument_id);
        if !matches!(argument, dir::Argument::Positional { .. }) {
            return None;
        }

        let expression_id = expression_unwrap_parenthesized(self.ctx.dir.tree(), expression_id);
        let expression = self.ctx.dir.get(expression_id);
        let dir::Expression::ScalarLiteral(dir::ScalarLiteral::RegexString { content, flags }) =
            expression
        else {
            return None;
        };
        let flags = flags.as_ref()?;
        let flags_text = self.ctx.strings.get(*flags);
        if !regex_flags_are_global_only(flags_text) {
            return None;
        }

        let pattern_text = self.ctx.strings.get(*content);
        let regex_parse = LintRegexParse::parse_with_flags(pattern_text, Some(flags_text));
        let hir = regex_parse.hir?;
        let literal_text = hir_literal_text(&hir)?;
        if literal_text.is_empty() {
            return None;
        }

        Some(single_quoted_string_literal(&literal_text))
    }
}

/// Return true when regex flags are exactly `g` with optional unicode mode.
fn regex_flags_are_global_only(flags: &str) -> bool {
    flags.chars().all(|flag| matches!(flag, 'g' | 'u' | 'v')) && flags.contains('g')
}

/// Return one plain literal text for regex HIR, or None for complex patterns.
fn hir_literal_text(hir: &regex_syntax::hir::Hir) -> Option<String> {
    match hir.kind() {
        HirKind::Literal(literal) => String::from_utf8(literal.0.to_vec()).ok(),
        HirKind::Concat(parts) => {
            let mut text = String::new();
            for part in parts {
                let part_text = hir_literal_text(part)?;
                text.push_str(&part_text);
            }

            Some(text)
        }
        _ => None,
    }
}

impl NodeVisitor for PreferStringReplaceAllVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check replace calls
        if let dir::Expression::Call {
            position: _,
            left,
            generic_arguments,
            arguments,
        } = expression
        {
            self.check_replace_call(id, *left, generic_arguments.as_slice(), arguments);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_global_regex_replace() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_flags_global_regex_replace.ds",
            r#"
let text = "hello";
let next = text.replace(/l/g, "x");
"#,
        );
        test.result(result).assert_lint("prefer-string-replaceall");
    }

    #[test]
    fn test_allows_non_global_regex_replace() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_allows_non_global_regex_replace.ds",
            r#"
let text = "hello";
let next = text.replace(/l/, "x");
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-string-replaceall");
    }

    #[test]
    fn test_fix_global_regex_replace() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_fix_global_regex_replace.ds",
            r#"
let text = "hello";
let next = text.replace(/l/g, "x");
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-replaceall")
            .assert_has_fix("prefer-string-replaceall")
            .assert_safe_fixed(
                r#"
let text = "hello";
let next = text.replaceAll('l', "x");
"#,
            );
    }

    #[test]
    fn test_fix_global_regex_replace_with_multiline_arguments() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_fix_global_regex_replace_with_multiline_arguments.ds",
            r#"
let text = "hello";
let next = text.replace(/l/g,
    "x"
);
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-replaceall")
            .assert_has_fix("prefer-string-replaceall")
            .assert_safe_fixed(
                r#"
let text = "hello";
let next = text.replaceAll('l', "x");
"#,
            );
    }

    #[test]
    fn test_flags_regexp_constructor_with_global_flag() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_flags_regexp_constructor_with_global_flag.ds",
            r#"
let text = "hello";
let next = text.replace(RegExp("l", "g"), "x");
"#,
        );
        test.result(result).assert_lint("prefer-string-replaceall");
    }

    #[test]
    fn test_flags_new_regexp_constructor_with_global_flag() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_flags_new_regexp_constructor_with_global_flag.ds",
            r#"
let text = "hello";
let next = text.replace(new RegExp("l", "gi"), "x");
"#,
        );
        test.result(result).assert_lint("prefer-string-replaceall");
    }

    /// Flag RegExp constructor calls when flags come from a static string alias.
    #[test]
    fn test_flags_regexp_constructor_with_static_string_flag_alias() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_flags_regexp_constructor_with_static_string_flag_alias.ds",
            r#"
let text = "hello";
const flags = "gi";
let next = text.replace(RegExp("l", flags), "x");
"#,
        );
        test.result(result).assert_lint("prefer-string-replaceall");
    }

    /// Allow RegExp constructor calls when static string alias flags are not global.
    #[test]
    fn test_allows_regexp_constructor_with_non_global_static_string_flag_alias() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_allows_regexp_constructor_with_non_global_static_string_flag_alias.ds",
            r#"
let text = "hello";
const flags = "i";
let next = text.replace(RegExp("l", flags), "x");
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-string-replaceall");
    }

    /// Flag RegExp constructor calls that inherit a global regex argument.
    #[test]
    fn test_flags_regexp_constructor_from_global_regex_argument() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_flags_regexp_constructor_from_global_regex_argument.ds",
            r#"
let text = "hello";
let next = text.replace(RegExp(/l/g), "x");
"#,
        );
        test.result(result).assert_lint("prefer-string-replaceall");
    }

    /// Allow RegExp constructor calls when explicit flags remove global matching.
    #[test]
    fn test_allows_regexp_constructor_with_non_global_override_flags() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_allows_regexp_constructor_with_non_global_override_flags.ds",
            r#"
let text = "hello";
let next = text.replace(RegExp(/l/g, "i"), "x");
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-string-replaceall");
    }

    #[test]
    fn test_no_fix_when_replace_contains_comments() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_no_fix_when_replace_contains_comments.ds",
            r#"
let text = "hello";
let next = text.replace(
    /l/g,
    /* replacement */ "x",
);
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-replaceall")
            .assert_has_no_fix("prefer-string-replaceall");
    }

    #[test]
    fn test_fix_replace_all_regex_pattern_to_string_literal() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_fix_replace_all_regex_pattern_to_string_literal.ds",
            r#"
let text = "hello";
let next = text.replaceAll(/l/g, "x");
"#,
        );
        test.result(result)
            .assert_lint("prefer-string-replaceall")
            .assert_has_fix("prefer-string-replaceall")
            .assert_safe_fixed(
                r#"
let text = "hello";
let next = text.replaceAll('l', "x");
"#,
            );
    }

    #[test]
    fn test_allows_replace_all_with_complex_regex_pattern() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_allows_replace_all_with_complex_regex_pattern.ds",
            r#"
let text = "hello";
let next = text.replaceAll(/l+/g, "x");
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-string-replaceall");
    }

    #[test]
    fn test_flags_replace_with_global_regex_variable() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_flags_replace_with_global_regex_variable.ds",
            r#"
let text = "hello";
const pattern = /l/g;
let next = text.replace(pattern, "x");
"#,
        );
        test.result(result).assert_lint("prefer-string-replaceall");
    }

    #[test]
    fn test_flags_replace_with_global_regex_alias_chain() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_flags_replace_with_global_regex_alias_chain.ds",
            r#"
let text = "hello";
const pattern = /l/g;
const alias = pattern;
let next = text.replace(alias, "x");
"#,
        );
        test.result(result).assert_lint("prefer-string-replaceall");
    }

    #[test]
    fn test_allows_replace_with_non_global_regex_variable() {
        let test = TestProgram::for_rule_without_prelude(PreferStringReplaceAll);
        let result = test.lint_dir(
            "prefer_string_replaceall/test_allows_replace_with_non_global_regex_variable.ds",
            r#"
let text = "hello";
const pattern = /l/;
let next = text.replace(pattern, "x");
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-string-replaceall");
    }
}
