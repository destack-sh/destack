use std::collections::HashSet;

use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{regex_pattern_info, regexp_global_qualifier_names};
use crate::{LintAstContext, LintDiagnostic, LintMeta, LintRule, declare_lint};

/// The set of accepted JavaScript regular expression flags.
const VALID_REGEX_FLAGS: [char; 8] = ['d', 'g', 'i', 'm', 's', 'u', 'v', 'y'];

declare_lint! {
    /// Disallow invalid regular expression strings.
    ///
    /// Invalid regular expressions will cause runtime errors. This rule
    /// catches syntax errors in regex literals at lint time.
    #[lint(
        id = "no-invalid-regexp",
        code = "LC020",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoInvalidRegexp,
    "Disallow invalid regular expressions"
}

impl LintRule for NoInvalidRegexp {
    fn meta(&self) -> &'static LintMeta {
        NoInvalidRegexp::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let regexp_name = ctx.string_id("RegExp");
        let global_qualifier_names = regexp_global_qualifier_names(ctx.strings);

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // resolve regex literal or constructor pattern info
            let Some(pattern_info) = regex_pattern_info(
                ctx.strings,
                ctx.tree,
                node_id,
                regexp_name,
                &global_qualifier_names,
            ) else {
                continue;
            };

            // report invalid flags when statically known
            if let Some(flags_id) = pattern_info.flags_id
                && let Some(flags_error) = invalid_regex_flags(ctx.strings.get(flags_id).as_ref())
            {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_INVALID_REGEXP.id,
                        NO_INVALID_REGEXP.code,
                        NO_INVALID_REGEXP.category,
                        severity,
                        format!("invalid regular expression flags: {flags_error}"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this regex flag set is invalid"),
                );

                continue;
            }

            // report invalid regex pattern parse errors
            let parse = ctx.regex_parse_with_flags(pattern_info.pattern_id, pattern_info.flags_id);

            // keep unknown constructor flags conservative:
            // only report when pattern is invalid in all relevant flag modes
            if pattern_info.has_unknown_flags {
                let Some(message) =
                    unknown_flags_pattern_error_message(ctx, pattern_info.pattern_id)
                else {
                    continue;
                };

                // resolve effective lint severity
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_INVALID_REGEXP.id,
                        NO_INVALID_REGEXP.code,
                        NO_INVALID_REGEXP.category,
                        severity,
                        format!("invalid regular expression: {message}"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this regex is invalid for all supported flag modes"),
                );

                continue;
            }

            let Some(parse_error) = parse.error.as_ref() else {
                continue;
            };

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    NO_INVALID_REGEXP.id,
                    NO_INVALID_REGEXP.code,
                    NO_INVALID_REGEXP.category,
                    severity,
                    format!("invalid regular expression: {}", parse_error.message),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("this regex is invalid"),
            );
        }
    }
}

/// Return one invalid regex flag reason, if present.
fn invalid_regex_flags(flags: &str) -> Option<String> {
    // track seen flags for duplicate detection
    let mut seen_flags = HashSet::new();
    for flag in flags.chars() {
        // reject unknown flags
        if !VALID_REGEX_FLAGS.contains(&flag) {
            return Some(format!("unknown flag `{flag}`"));
        }

        // reject duplicate flags
        if !seen_flags.insert(flag) {
            return Some(format!("duplicate flag `{flag}`"));
        }
    }

    // reject mutually exclusive unicode modes
    if seen_flags.contains(&'u') && seen_flags.contains(&'v') {
        return Some("flags `u` and `v` are mutually exclusive".to_string());
    }

    None
}

/// Return one parse error message if a pattern is invalid for all unknown constructor flag modes.
fn unknown_flags_pattern_error_message(
    ctx: &mut LintAstContext<'_>,
    pattern_id: ast::StringId,
) -> Option<String> {
    let pattern_has_set_notation = {
        let pattern_text = ctx.strings.get(pattern_id);
        pattern_text.contains('{') || pattern_text.contains('}')
    };

    // keep unknown flags conservative for `{` and `}` because JS `/v` set notation
    // changes parse semantics and `regex_syntax` does not fully model those forms
    if pattern_has_set_notation {
        return None;
    }

    let unicode_flags = ctx.string_id("u");
    let unicode_sets_flags = ctx.string_id("v");

    let default_parse = ctx.regex_parse_with_flags(pattern_id, None);
    let unicode_parse = ctx.regex_parse_with_flags(pattern_id, Some(unicode_flags));
    let unicode_sets_parse = ctx.regex_parse_with_flags(pattern_id, Some(unicode_sets_flags));

    let default_error = default_parse.error.as_ref()?;
    let unicode_error = unicode_parse.error.as_ref()?;
    let unicode_sets_error = unicode_sets_parse.error.as_ref()?;

    // choose one stable message when all modes fail
    let message = if !default_error.message.is_empty() {
        default_error.message.clone()
    } else if !unicode_error.message.is_empty() {
        unicode_error.message.clone()
    } else {
        unicode_sets_error.message.clone()
    };

    Some(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_invalid_regex_unmatched_paren() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_detects_invalid_regex_unmatched_paren.ds",
            r#"
let re = /(/
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_invalid_regex_invalid_group() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_detects_invalid_regex_invalid_group.ds",
            r#"
let re = /(?/
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_invalid_regex_incomplete_escape() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_detects_invalid_regex_incomplete_escape.ds",
            r#"
let re = /\p/
"#,
        );
        // regex syntax considers \p incomplete, missing property name
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_allows_valid_regex() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_allows_valid_regex.ds",
            r#"
let re = /^[a-z]+$/
"#,
        );
        test.result(result).assert_no_lint("no-invalid-regexp");
    }

    #[test]
    fn test_allows_complex_valid_regex() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_allows_complex_valid_regex.ds",
            r#"
let re = /(\d{1,3}\.){3}\d{1,3}/
"#,
        );
        test.result(result).assert_no_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_invalid_repetition() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_detects_invalid_repetition.ds",
            r#"
let re = /a{3,1}/
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_invalid_regexp_constructor_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_detects_invalid_regexp_constructor_pattern.ds",
            r#"
let re = RegExp("(")
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_invalid_regexp_constructor_flags() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_detects_invalid_regexp_constructor_flags.ds",
            r#"
let re = new RegExp("ok", "gg")
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_allows_regexp_constructor_with_unknown_flags_expression() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_allows_regexp_constructor_with_unknown_flags_expression.ds",
            r#"
let flags = "g";
let re = RegExp("ok", flags)
"#,
        );
        test.result(result).assert_no_lint("no-invalid-regexp");
    }

    #[test]
    fn test_allows_unknown_flags_for_pattern_that_depends_on_runtime_mode() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_allows_unknown_flags_for_pattern_that_depends_on_runtime_mode.ds",
            r#"
let flags = resolveFlags();
let re = RegExp("{", flags)
"#,
        );
        test.result(result).assert_no_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_unknown_flags_when_pattern_invalid_in_all_modes() {
        let test = TestProgram::for_rule_without_prelude(NoInvalidRegexp);
        let result = test.lint_ast(
            "no_invalid_regexp/test_detects_unknown_flags_when_pattern_invalid_in_all_modes.ds",
            r#"
let flags = resolveFlags();
let re = RegExp("(", flags)
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }
}
