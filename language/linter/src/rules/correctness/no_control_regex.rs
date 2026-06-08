use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::{regex_pattern_info, regexp_global_qualifier_names};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow control characters in regular expressions.
    ///
    /// Control characters (ASCII codes 0x00-0x1F) are rarely useful in
    /// regular expressions and are often the result of a typo. They can
    /// also cause unexpected behavior.
    #[lint(
        id = "no-control-regex",
        code = "LC010",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoControlRegex,
    "Disallow control characters in regex"
}

impl LintRule for NoControlRegex {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoControlRegex::meta()
    }

    /// Check module source nodes for regex patterns containing control characters.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let regexp_name = ctx.string_id("RegExp");
        let global_qualifier_names = regexp_global_qualifier_names(ctx.strings);

        // walk expression nodes
        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let Some(pattern_info) = regex_pattern_info(
                ctx.strings,
                ctx.dir.tree(),
                node_id,
                regexp_name,
                &global_qualifier_names,
            ) else {
                continue;
            };

            // resolve all control characters from pattern and flag semantics
            let control_characters =
                ctx.regex_control_characters(pattern_info.pattern_id, pattern_info.flags_id);
            if control_characters.is_empty() {
                continue;
            }

            // resolve effective severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // report diagnostic
            ctx.report(
                LintReport::new(
                    NO_CONTROL_REGEX.id,
                    NO_CONTROL_REGEX.code,
                    NO_CONTROL_REGEX.category,
                    severity,
                    format!(
                        "unexpected control character(s) in regular expression: {}",
                        control_characters.join(", ")
                    ),
                    ctx.dir.get_span(node_id),
                )
                .label("control characters are rarely intended"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_control_char_in_regex() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        // use a hex escape to embed a control character
        let result = test.lint(
            "no_control_regex/test_detects_control_char_in_regex.ds",
            "/\x01/",
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_allows_normal_regex() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_allows_normal_regex.ds",
            r#"
let re = /abc/
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }

    #[test]
    fn test_allows_escaped_control_sequences() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_allows_escaped_control_sequences.ds",
            r#"
let re = /\n\t\r/
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }

    #[test]
    fn test_detects_hex_escapes() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_detects_hex_escapes.ds",
            r#"
let re = /\x00\x1F/
"#,
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_detects_control_char_in_regexp_call_literal() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_detects_control_char_in_regexp_call_literal.ds",
            "RegExp(\"\x01\");",
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_detects_control_char_in_new_regexp_literal() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_detects_control_char_in_new_regexp_literal.ds",
            "new RegExp(\"\x01\");",
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_detects_unicode_escape_in_regex_literal() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_detects_unicode_escape_in_regex_literal.ds",
            r#"
let re = /\u001F/
"#,
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_allows_unicode_code_point_escape_without_unicode_flag() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_allows_unicode_code_point_escape_without_unicode_flag.ds",
            r#"
let re = /\u{1F}/
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }

    #[test]
    fn test_detects_unicode_code_point_escape_with_u_flag() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_detects_unicode_code_point_escape_with_u_flag.ds",
            r#"
let re = /\u{1F}/u
"#,
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_allows_unicode_code_point_escape_with_unknown_constructor_flags() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_allows_unicode_code_point_escape_with_unknown_constructor_flags.ds",
            r#"
let flags = "u";
RegExp("\\u{1F}", flags);
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }

    #[test]
    fn test_allows_non_regexp_constructor_calls() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint(
            "no_control_regex/test_allows_non_regexp_constructor_calls.ds",
            "buildRegExp(\"\\x01\");",
        );
        test.result(result).assert_no_lint("no-control-regex");
    }
}
