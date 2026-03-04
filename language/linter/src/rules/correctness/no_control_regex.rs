use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::ast_regex_pattern_info;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        NoControlRegex::meta()
    }

    /// Check module AST nodes for regex patterns containing control characters.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();
        let regexp_name = ctx.strings.intern("RegExp");

        // walk expression nodes
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let Some(pattern_info) = ast_regex_pattern_info(ctx.tree, node_id, regexp_name) else {
                continue;
            };

            // resolve control character from pattern text
            let Some(control_char) = ctx.regex_control_character(pattern_info.pattern_id) else {
                continue;
            };

            // resolve effective severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // report diagnostic
            ctx.report(
                LintDiagnostic::new(
                    NO_CONTROL_REGEX.id,
                    NO_CONTROL_REGEX.code,
                    NO_CONTROL_REGEX.category,
                    severity,
                    format!(
                        "unexpected control character in regular expression: \\x{:02X}",
                        control_char as u8
                    ),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("control characters are rarely intended"),
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
        let result = test.lint_ast(
            "no_control_regex/test_detects_control_char_in_regex.ds",
            "/\x01/",
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_allows_normal_regex() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint_ast(
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
        let result = test.lint_ast(
            "no_control_regex/test_allows_escaped_control_sequences.ds",
            r#"
let re = /\n\t\r/
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }

    #[test]
    fn test_allows_hex_escapes() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint_ast(
            "no_control_regex/test_allows_hex_escapes.ds",
            r#"
let re = /\x00\x1F/
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }

    #[test]
    fn test_detects_control_char_in_regexp_call_literal() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint_ast(
            "no_control_regex/test_detects_control_char_in_regexp_call_literal.ds",
            "RegExp(\"\x01\");",
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_detects_control_char_in_new_regexp_literal() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint_ast(
            "no_control_regex/test_detects_control_char_in_new_regexp_literal.ds",
            "new RegExp(\"\x01\");",
        );
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_allows_non_regexp_constructor_calls() {
        let test = TestProgram::for_rule_without_prelude(NoControlRegex);
        let result = test.lint_ast(
            "no_control_regex/test_allows_non_regexp_constructor_calls.ds",
            "buildRegExp(\"\\x01\");",
        );
        test.result(result).assert_no_lint("no-control-regex");
    }
}
