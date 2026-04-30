use crate::rules::common::{regex_pattern_info, regexp_global_qualifier_names};
use crate::{LintAstContext, LintDiagnostic, LintMeta, LintRule, declare_lint};
use destack_ast as ast;
use destack_workspace::LintSeverity;

declare_lint! {
    /// Disallow useless backreferences in regular expressions.
    ///
    /// Backreferences that reference non existent groups or forward references
    /// groups that haven't been captured yet will never match anything useful.
    #[lint(
        id = "no-useless-backreference",
        code = "LU034",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessBackreference,
    "Disallow useless regex backreferences"
}

impl LintRule for NoUselessBackreference {
    fn meta(&self) -> &'static LintMeta {
        NoUselessBackreference::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let regexp_name = ctx.string_id("RegExp");
        let global_qualifier_names = regexp_global_qualifier_names(ctx.strings);

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let Some(pattern_info) = regex_pattern_info(
                ctx.strings,
                ctx.tree,
                node_id,
                regexp_name,
                &global_qualifier_names,
            ) else {
                continue;
            };

            let problem =
                ctx.regex_useless_backreference(pattern_info.pattern_id, pattern_info.flags_id);
            let Some(problem) = problem.as_deref() else {
                continue;
            };
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    NO_USELESS_BACKREFERENCE.id,
                    NO_USELESS_BACKREFERENCE.code,
                    NO_USELESS_BACKREFERENCE.category,
                    severity,
                    problem,
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("this backreference will never match"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nested_backreference() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_detects_nested_backreference.ds",
            r#"
const re = /(b)(\2a)/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_forward_reference() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_detects_forward_reference.ds",
            r#"
const re = /\1(a)/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_named_forward_reference() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_detects_named_forward_reference.ds",
            r#"
const re = /\k<foo>(?<foo>a)/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_backward_reference_in_lookbehind() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_detects_backward_reference_in_lookbehind.ds",
            r#"
const re = /(?<=(a)\1)b/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_disjunctive_reference() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_detects_disjunctive_reference.ds",
            r#"
const re = /(a)|\1b/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_reference_into_negative_lookaround() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_detects_reference_into_negative_lookaround.ds",
            r#"
const re = /a(?!(b)).\1/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_forward_style_reference_inside_lookbehind() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_forward_style_reference_inside_lookbehind.ds",
            r#"
const re = /(?<=\1(a))b/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_valid_backreference() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_valid_backreference.ds",
            r#"
const re = /(a)\1/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_valid_named_backreference() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_valid_named_backreference.ds",
            r#"
const re = /(?<foo>a)\k<foo>/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_multiple_valid_backreferences() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_multiple_valid_backreferences.ds",
            r#"
const re = /(a)(b)\1\2/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_disjunctive_backreference_in_same_alternative() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_disjunctive_backreference_in_same_alternative.ds",
            r#"
const re = /^(a)|(b)\2$/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_named_backreference_with_one_valid_alternative() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_named_backreference_with_one_valid_alternative.ds",
            r#"
const re = /((?<foo>bar)\k<foo>|(?<foo>baz))/;
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_named_forward_reference_across_alternatives() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_detects_named_forward_reference_across_alternatives.ds",
            r#"
const re = /\k<foo>((?<foo>bar)|(?<foo>baz))/;
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_regex_without_backreference() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_regex_without_backreference.ds",
            r#"
const re = /hello/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_escaped_digit_in_char_class() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_escaped_digit_in_char_class.ds",
            r#"
const re = /[\1]/
"#,
        );
        // \1 in character class is octal, not backreference
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_octal_escape_when_group_does_not_exist() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_octal_escape_when_group_does_not_exist.ds",
            r#"
const re = /(a)\2/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_ignores_backreference_when_pattern_has_other_parse_error() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_ignores_backreference_when_pattern_has_other_parse_error.ds",
            r#"
const re = RegExp("\\1(a)[", "u");
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_ignores_backreference_when_pattern_has_unclosed_quantifier_error() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_ignores_backreference_when_pattern_has_unclosed_quantifier_error.ds",
            r#"
const re = RegExp("\\1(a){", "u");
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_forward_backreference_in_regexp_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_detects_forward_backreference_in_regexp_constructor.ds",
            r#"
const re = RegExp("\\1(a)");
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_valid_backreference_in_regexp_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessBackreference);
        let result = test.lint_ast(
            "no_useless_backreference/test_allows_valid_backreference_in_regexp_constructor.ds",
            r#"
const re = RegExp("(a)\\1");
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }
}
