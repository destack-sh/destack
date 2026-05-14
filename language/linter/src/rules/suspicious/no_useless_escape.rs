use destack_dir as dir;
use destack_source::Span;
use destack_workspace::LintSeverity;
use std::collections::HashSet;

use crate::rules::common::{regex_pattern_info, regexp_global_qualifier_names};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary escape characters in strings and regex literals.
    ///
    /// Escaping characters that do not need escaping is noisy and can hide
    /// mistakes in string and regex intent.
    #[lint(
        id = "no-useless-escape",
        code = "LU039",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessEscape,
    "Disallow useless escape characters"
}

impl LintRule for NoUselessEscape {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUselessEscape::meta()
    }

    /// Check module source nodes for useless escapes.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve metadata and source text once
        let meta = self.meta();
        let source = ctx.file.text().to_string();
        let source = source.as_str();
        let regexp_name = ctx.string_id("RegExp");
        let global_qualifier_names = regexp_global_qualifier_names(ctx.strings);
        let allowed_regex_escape_characters = configured_regex_escape_characters(
            &ctx.options
                .correctness
                .no_useless_escape_allow_regex_characters,
        );

        // inspect candidate expression literals
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let span = ctx.dir.get_span(node_id);

            // report string and template literal escapes
            if matches!(
                expression,
                dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(_))
                    | dir::Expression::TemplateExpression { .. }
            ) {
                report_string_literal_escapes(ctx, meta, node_id, span, source);
                continue;
            }

            // report regex literal and constructor escapes
            let Some(pattern_info) = regex_pattern_info(
                ctx.strings,
                ctx.dir.tree(),
                node_id,
                regexp_name,
                &global_qualifier_names,
            ) else {
                continue;
            };
            report_regex_escapes(
                ctx,
                meta,
                node_id,
                span,
                source,
                pattern_info.pattern_id,
                pattern_info.flags_id,
                pattern_info.has_unknown_flags,
                &allowed_regex_escape_characters,
            );
        }
    }
}

/// Valid string escapes for this rule.
const VALID_STRING_ESCAPES: &[char] = &[
    'n', 'r', 't', 'b', 'f', 'v', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '\\', '\'',
    '"', '`', 'x', 'u', '\n', '\r',
];

/// Valid regex escapes in and outside character classes.
const REGEX_GENERAL_ESCAPES: &[char] = &[
    '\\', 'b', 'c', 'd', 'D', 'f', 'n', 'p', 'P', 'r', 's', 'S', 't', 'v', 'w', 'W', 'x', 'u', '0',
    '1', '2', '3', '4', '5', '6', '7', '8', '9', ']',
];

/// Valid regex escapes outside character classes.
const REGEX_NON_CLASS_ESCAPES: &[char] = &[
    '\\', 'b', 'c', 'd', 'D', 'f', 'n', 'p', 'P', 'r', 's', 'S', 't', 'v', 'w', 'W', 'x', 'u', '0',
    '1', '2', '3', '4', '5', '6', '7', '8', '9', ']', '^', '/', '.', '$', '*', '+', '?', '[', '{',
    '}', '|', '(', ')', 'B', 'k',
];

/// Valid regex escapes inside unicode set classes.
const REGEX_CLASS_SET_ESCAPES: &[char] = &[
    '\\', 'b', 'c', 'd', 'D', 'f', 'n', 'p', 'P', 'r', 's', 'S', 't', 'v', 'w', 'W', 'x', 'u', '0',
    '1', '2', '3', '4', '5', '6', '7', '8', '9', ']', 'q', '/', '[', '{', '}', '|', '(', ')', '-',
];

/// Reserved double punctuator characters in unicode set classes.
const REGEX_CLASS_SET_RESERVED_DOUBLE_PUNCTUATOR: &[char] = &[
    '!', '#', '$', '%', '&', '*', '+', ',', '.', ':', ';', '<', '=', '>', '?', '@', '^', '`', '~',
];

/// Report useless escapes in one string or template literal node.
fn report_string_literal_escapes(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    node_id: dir::LocalNodeId<dir::Expression>,
    literal_span: Span,
    source: &str,
) {
    // resolve raw literal source slice
    let start = literal_span.start as usize;
    let end = literal_span.end as usize;
    if start >= source.len() || end > source.len() {
        return;
    }
    let raw_literal = &source[start..end];

    // collect all useless escape positions
    let escape_positions = find_useless_string_escape_positions(raw_literal);
    if escape_positions.is_empty() {
        return;
    }

    // resolve effective severity once per node
    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    // report each useless backslash independently
    for backslash_position in escape_positions {
        let Some(escape_character) = raw_literal[backslash_position + 1..].chars().next() else {
            continue;
        };

        let mut diagnostic = LintReport::new(
            NO_USELESS_ESCAPE.id,
            NO_USELESS_ESCAPE.code,
            NO_USELESS_ESCAPE.category,
            severity,
            format!("unnecessary escape character: \\{escape_character}"),
            literal_span,
        )
        .label("this escape is unnecessary");

        // attach one safe backslash removal fix
        if ctx.compute_fixes
            && let Some(backslash_span) =
                string_backslash_span(literal_span, raw_literal, backslash_position)
        {
            let edits = ctx.edit_builder().delete(backslash_span).into_edits();
            let fix = LintFix::suggestion("Remove unnecessary escape backslash").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// Report useless escapes in one regex literal node.
fn report_regex_escapes(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    node_id: dir::LocalNodeId<dir::Expression>,
    literal_span: Span,
    source: &str,
    pattern_id: dir::StringId,
    flags_id: Option<dir::StringId>,
    has_unknown_flags: bool,
    allowed_regex_escape_characters: &HashSet<char>,
) {
    // resolve regex pattern and flags
    let pattern = ctx.strings.get(pattern_id);
    let flags = flags_id.map(|id| ctx.strings.get(id));
    let flags = if has_unknown_flags {
        // treat unknown runtime flags as no flags
        None
    } else {
        flags
    };

    // collect useless regex escapes from the pattern text
    let escape_positions =
        find_useless_regex_escape_positions(pattern, flags, allowed_regex_escape_characters);
    if escape_positions.is_empty() {
        return;
    }

    // resolve effective severity once per node
    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    // resolve raw regex text once for fix span mapping
    let start = literal_span.start as usize;
    let end = literal_span.end as usize;
    if start >= source.len() || end > source.len() {
        return;
    }
    let raw_literal = &source[start..end];

    // report each useless regex backslash independently
    for backslash_position in escape_positions {
        let Some(escape_character) = pattern[backslash_position + 1..].chars().next() else {
            continue;
        };

        let mut diagnostic = LintReport::new(
            NO_USELESS_ESCAPE.id,
            NO_USELESS_ESCAPE.code,
            NO_USELESS_ESCAPE.category,
            severity,
            format!("unnecessary escape character: \\{escape_character}"),
            literal_span,
        )
        .label("this escape is unnecessary");

        // attach one safe backslash removal fix
        if ctx.compute_fixes
            && let Some(backslash_span) =
                regex_backslash_span(literal_span, raw_literal, backslash_position)
        {
            let edits = ctx.edit_builder().delete(backslash_span).into_edits();
            let fix = LintFix::suggestion("Remove unnecessary escape backslash").with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// Resolve one useless string escape position list from one raw literal.
fn find_useless_string_escape_positions(raw_literal: &str) -> Vec<usize> {
    // resolve one opening quote marker
    let mut positions = Vec::new();
    let mut characters = raw_literal.char_indices().peekable();
    let Some((_, quote)) = characters.next() else {
        return positions;
    };
    if !matches!(quote, '"' | '\'' | '`') {
        return positions;
    }

    // walk backslash escape pairs
    while let Some((backslash_position, character)) = characters.next() {
        if character != '\\' {
            continue;
        }

        let Some((_, escaped_character)) = characters.peek() else {
            continue;
        };

        // keep escaped template interpolation delimiters
        if quote == '`'
            && template_interpolation_escape_is_intentional(
                raw_literal,
                backslash_position,
                *escaped_character,
            )
        {
            characters.next();
            continue;
        }

        // keep quote escapes and known escape classes
        if *escaped_character == quote || VALID_STRING_ESCAPES.contains(escaped_character) {
            characters.next();
            continue;
        }

        positions.push(backslash_position);
        characters.next();
    }

    positions
}

/// Return true when one template escape pair is intentionally preserving `${...}` text.
fn template_interpolation_escape_is_intentional(
    raw_literal: &str,
    backslash_position: usize,
    escaped_character: char,
) -> bool {
    // keep `\${...}` escapes
    if escaped_character == '$'
        && raw_literal
            .get(backslash_position + 2..)
            .and_then(|suffix| suffix.chars().next())
            .is_some_and(|character| character == '{')
    {
        return true;
    }

    // keep `$\{...}` escapes
    escaped_character == '{'
        && raw_literal
            .get(..backslash_position)
            .and_then(|prefix| prefix.chars().next_back())
            .is_some_and(|character| character == '$')
}

/// Resolve one useless regex escape position list from one pattern and flags.
fn find_useless_regex_escape_positions(
    pattern: &str,
    flags: Option<&str>,
    allowed_regex_escape_characters: &HashSet<char>,
) -> Vec<usize> {
    let unicode_set_mode = flags.is_some_and(|flag_text| flag_text.contains('v'));

    // walk regex pattern characters
    let mut positions = Vec::new();
    let indexed_characters: Vec<_> = pattern.char_indices().collect();
    let mut index = 0;
    let mut class_content_starts = Vec::new();
    let mut class_is_negated_stack = Vec::new();

    while index < indexed_characters.len() {
        let (_, character) = indexed_characters[index];

        // keep class boundary tracking outside escapes
        if character == '[' && (class_content_starts.is_empty() || unicode_set_mode) {
            let class_content_start = index + 1;
            let class_is_negated = indexed_characters
                .get(class_content_start)
                .is_some_and(|(_, character)| *character == '^');
            class_content_starts.push(class_content_start);
            class_is_negated_stack.push(class_is_negated);
            index += 1;
            continue;
        }
        if character == ']' && !class_content_starts.is_empty() {
            class_content_starts.pop();
            class_is_negated_stack.pop();
            index += 1;
            continue;
        }

        // process one escape pair
        if character == '\\' && index + 1 < indexed_characters.len() {
            let escaped_character = indexed_characters[index + 1].1;
            if allowed_regex_escape_characters.contains(&escaped_character) {
                index += 2;
                continue;
            }

            let escape_is_valid = if let Some(class_content_start) = class_content_starts.last() {
                let class_is_negated = class_is_negated_stack.last().copied().unwrap_or(false);
                regex_character_class_escape_is_valid(
                    &indexed_characters,
                    index,
                    *class_content_start,
                    class_is_negated,
                    unicode_set_mode,
                    escaped_character,
                )
            } else {
                REGEX_NON_CLASS_ESCAPES.contains(&escaped_character)
            };

            if !escape_is_valid {
                positions.push(indexed_characters[index].0);
            }

            index += 2;
            continue;
        }

        index += 1;
    }

    positions
}

/// Return configured regex escape characters as a character set.
fn configured_regex_escape_characters(values: &[String]) -> HashSet<char> {
    values
        .iter()
        .filter_map(|value| value.chars().next())
        .collect()
}

/// Return true when one escaped regex character is valid inside a character class.
fn regex_character_class_escape_is_valid(
    indexed_characters: &[(usize, char)],
    backslash_index: usize,
    class_content_start: usize,
    class_is_negated: bool,
    unicode_set_mode: bool,
    escaped_character: char,
) -> bool {
    // keep unicode set mode semantics for character class escapes
    if unicode_set_mode {
        if REGEX_CLASS_SET_ESCAPES.contains(&escaped_character) {
            return true;
        }

        return unicode_set_reserved_double_escape_is_valid(
            indexed_characters,
            backslash_index,
            class_content_start,
            class_is_negated,
            escaped_character,
        );
    }

    // keep general valid class escapes
    if REGEX_GENERAL_ESCAPES.contains(&escaped_character) {
        return true;
    }

    // keep escaped caret only at class start
    if escaped_character == '^' {
        return backslash_index == class_content_start;
    }

    // keep escaped dash only in class middle positions
    if escaped_character == '-' {
        let is_first = backslash_index == class_content_start;
        let is_last = indexed_characters
            .get(backslash_index + 2)
            .is_none_or(|(_, character)| *character == ']');
        return !is_first && !is_last;
    }

    false
}

/// Return true when one unicode set class escape is one reserved double punctuator escape.
fn unicode_set_reserved_double_escape_is_valid(
    indexed_characters: &[(usize, char)],
    backslash_index: usize,
    class_content_start: usize,
    class_is_negated: bool,
    escaped_character: char,
) -> bool {
    // keep reserved punctuator escapes only when the punctuator is doubled
    if !REGEX_CLASS_SET_RESERVED_DOUBLE_PUNCTUATOR.contains(&escaped_character) {
        return false;
    }

    let next_is_same = indexed_characters
        .get(backslash_index + 2)
        .is_some_and(|(_, character)| *character == escaped_character);
    if next_is_same {
        return true;
    }

    let previous_is_same = backslash_index
        .checked_sub(1)
        .and_then(|index| indexed_characters.get(index))
        .is_some_and(|(_, character)| *character == escaped_character);
    if !previous_is_same {
        return false;
    }
    if escaped_character != '^' {
        return true;
    }
    if !class_is_negated {
        return true;
    }

    let negate_caret_index = class_content_start;
    negate_caret_index < backslash_index.saturating_sub(1)
}

/// Resolve one absolute source span for a string literal backslash.
fn string_backslash_span(
    literal_span: Span,
    raw_literal: &str,
    backslash_position: usize,
) -> Option<Span> {
    // keep position bounds valid
    if backslash_position >= raw_literal.len() {
        return None;
    }

    // resolve one absolute byte span
    let absolute_start = literal_span.start + backslash_position as u32;
    Some(Span::new(
        literal_span.file,
        absolute_start,
        absolute_start + 1,
    ))
}

/// Resolve one absolute source span for a regex literal backslash.
fn regex_backslash_span(
    literal_span: Span,
    raw_literal: &str,
    pattern_backslash_position: usize,
) -> Option<Span> {
    // require slash delimited regex literal form
    if !raw_literal.starts_with('/') {
        return None;
    }
    if pattern_backslash_position >= raw_literal.len().saturating_sub(1) {
        return None;
    }

    // resolve one absolute byte span after the opening slash
    let absolute_start = literal_span.start + 1 + pattern_backslash_position as u32;
    Some(Span::new(
        literal_span.file,
        absolute_start,
        absolute_start + 1,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_useless_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_detects_useless_escape.ds",
            r#"
const x = "hel\lo"
"#,
        );
        test.result(result)
            .assert_lint("no-useless-escape")
            .assert_has_fix("no-useless-escape");
    }

    #[test]
    fn test_allows_valid_escapes() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_valid_escapes.ds",
            r#"
const x = "hello\nworld"
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_quote_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_quote_escape.ds",
            r#"
const x = "say \"hello\""
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_backslash_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_backslash_escape.ds",
            r#"
const x = "path\\to\\file"
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_fix_removes_useless_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_fix_removes_useless_escape.ds",
            r#"
const x = "hel\lo"
"#,
        );
        test.result(result)
            .assert_lint("no-useless-escape")
            .assert_suggested_fixed(
                r#"
const x = "hello";
"#,
            );
    }

    #[test]
    fn test_fix_preserves_valid_escapes_and_removes_only_useless_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_fix_preserves_valid_escapes_and_removes_only_useless_escape.ds",
            r#"
const x = "hel\lo\n"
"#,
        );
        test.result(result)
            .assert_lint("no-useless-escape")
            .assert_suggested_fixed(
                r#"
const x = "hello\n";
"#,
            );
    }

    #[test]
    fn test_mutation_detects_useless_escape_in_single_quote() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_mutation_detects_useless_escape_in_single_quote.ds",
            r#"
const x = 'ab\cd'
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_mutation_detects_useless_escape_in_double_quote() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_mutation_detects_useless_escape_in_double_quote.ds",
            r#"
const x = "ab\cd"
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_detects_useless_escape_in_template_literal() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_detects_useless_escape_in_template_literal.ds",
            r#"
const x = `ab\cd`
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_template_interpolation_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_template_interpolation_escape.ds",
            r#"
const x = `\${name}`
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_valid_regex_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_valid_regex_escape.ds",
            r#"
const re = /\./;
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_detects_useless_regex_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_detects_useless_regex_escape.ds",
            r#"
const re = /\a/;
"#,
        );
        test.result(result)
            .assert_lint("no-useless-escape")
            .assert_suggested_fixed(
                r#"
const re = /a/;
"#,
            );
    }

    #[test]
    fn test_detects_useless_regex_escape_in_character_class() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_detects_useless_regex_escape_in_character_class.ds",
            r#"
const re = /[a\?]/;
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_valid_unicode_set_class_double_punctuator_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_valid_unicode_set_class_double_punctuator_escape.ds",
            r#"
const re = /[\&&]/v;
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_detects_useless_unicode_set_class_single_punctuator_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_detects_useless_unicode_set_class_single_punctuator_escape.ds",
            r#"
const re = /[\&a]/v;
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_valid_unicode_set_class_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_valid_unicode_set_class_escape.ds",
            r#"
const re = /[\(]/v;
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_nested_unicode_set_class_dash_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_nested_unicode_set_class_dash_escape.ds",
            r#"
const re = /[[\-]\-]/v;
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_unicode_set_negated_double_caret_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_unicode_set_negated_double_caret_escape.ds",
            r#"
const re = /[^\^^]/v;
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_unicode_set_caret_with_previous_caret_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_unicode_set_caret_with_previous_caret_escape.ds",
            r#"
const re = /[_\^^]/v;
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_detects_useless_unicode_set_single_caret_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_detects_useless_unicode_set_single_caret_escape.ds",
            r#"
const re = /[^\^]/v;
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_unicode_set_reserved_double_punctuator_chain_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_unicode_set_reserved_double_punctuator_chain_escape.ds",
            r#"
const re = /[\&&&\&]/v;
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_detects_useless_regex_escape_in_regexp_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_detects_useless_regex_escape_in_regexp_constructor.ds",
            r#"
const re = RegExp("\a");
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_detects_useless_regex_escape_with_unknown_regexp_flags() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_detects_useless_regex_escape_with_unknown_regexp_flags.ds",
            r#"
const flags = "u";
const re = RegExp("\a", flags);
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_valid_regex_escape_in_regexp_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint(
            "no_useless_escape/test_allows_valid_regex_escape_in_regexp_constructor.ds",
            r#"
const re = RegExp("\\.");
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_configured_regex_escape_character() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape).with_options(|options| {
            options.correctness.no_useless_escape_allow_regex_characters = vec!["-".to_string()];
        });
        let result = test.lint(
            "no_useless_escape/test_allows_configured_regex_escape_character.ds",
            r#"
const pattern = /[\-]/
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }
}
