use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_path_segments, expression_unwrap_parenthesized_syntax, path_is_regexp_constructor,
    regex_pattern_info, regexp_global_qualifier_names,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require `u` or `v` flag on regular expressions.
    ///
    /// The `u` (unicode) flag enables correct handling of Unicode characters
    /// in regular expressions. Without it, patterns may not match characters
    /// outside the Basic Multilingual Plane correctly, and character classes
    /// like `\w` won't match non ASCII letters.
    ///
    /// The `v` flag (unicodeSets) is a more powerful alternative that also
    /// enables set notation and properties of strings.
    ///
    /// bad: `/foo/`
    /// good: `/foo/u` or `/foo/v`
    #[lint(
        id = "require-unicode-regexp",
        code = "LP018",
        category = Performance,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireUnicodeRegexp,
    "Require unicode flag on regex"
}

impl LintRule for RequireUnicodeRegexp {
    fn meta(&self) -> &'static LintMeta {
        RequireUnicodeRegexp::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let regexp_name = ctx.strings.intern("RegExp");
        let global_qualifier_names = regexp_global_qualifier_names(ctx.strings);

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // resolve regex literals and static RegExp constructor patterns
            let Some(regex_info) = regex_pattern_info(
                ctx.strings,
                ctx.tree,
                node_id,
                regexp_name,
                &global_qualifier_names,
            ) else {
                continue;
            };

            // skip constructor calls when flags are present but not statically known
            if has_unknown_constructor_flags_argument(
                ctx,
                node_id,
                regexp_name,
                &global_qualifier_names,
            ) {
                continue;
            }

            // check if flags contain 'u' or 'v'
            let has_unicode_flag = if let Some(flags_id) = regex_info.flags_id {
                let flags_str = ctx.strings.get(flags_id);
                let flags_ref = flags_str.as_ref();
                flags_ref.contains('u') || flags_ref.contains('v')
            } else {
                false
            };
            if !has_unicode_flag {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let mut diagnostic = LintDiagnostic::new(
                    REQUIRE_UNICODE_REGEXP.id,
                    REQUIRE_UNICODE_REGEXP.code,
                    REQUIRE_UNICODE_REGEXP.category,
                    severity,
                    "regex should have the 'u' or 'v' flag for proper Unicode handling",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("add the 'u' flag for Unicode support");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) =
                        unicode_regex_fix(ctx, node_id, regexp_name, &global_qualifier_names)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return true when this is a RegExp constructor with unknown flags.
fn has_unknown_constructor_flags_argument(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
    regexp_name: ast::StringId,
    global_qualifier_names: &[ast::StringId],
) -> bool {
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);
    let (callee_id, arguments) = match expression {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => (*left, dynamic_arguments.as_slice()),
        _ => return false,
    };

    // require optional structure
    let Some(path_segments) = expression_path_segments(ctx.tree, callee_id) else {
        return false;
    };
    if !path_is_regexp_constructor(
        path_segments.as_slice(),
        regexp_name,
        global_qualifier_names,
    ) {
        return false;
    }

    // enforce this lint guard
    if arguments.len() < 2 {
        return false;
    }

    // resolve second argument
    let second_argument = ctx.tree.get(arguments[1]);
    let ast::Argument::Positional { value, .. } = second_argument else {
        return true;
    };
    let second_expression = ctx.tree.get(*value);
    !matches!(
        second_expression,
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    )
}

/// Build a safe fix that appends a unicode flag to a regex literal.
fn unicode_regex_fix(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
    regexp_name: ast::StringId,
    global_qualifier_names: &[ast::StringId],
) -> Option<LintFix> {
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);

    // fix regex literals by appending `u`
    if matches!(
        expression,
        Expression::ScalarLiteral(ScalarLiteral::RegexString { .. })
    ) {
        let expression_span = ctx.tree.get_span(expression_id);
        let expression_text = ctx.get_span_text(expression_span);
        let expression_text: &str = expression_text;
        if expression_text.is_empty() {
            return None;
        }

        // build replacement text
        let replacement = format!("{expression_text}u");
        let edits = ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();
        return Some(LintFix::safe("Add unicode regex flag").with_edits(edits));
    }

    // fix RegExp constructors by adding or extending the flags argument
    let (callee_id, arguments) = match expression {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => (*left, dynamic_arguments.as_slice()),
        _ => return None,
    };
    let path_segments = expression_path_segments(ctx.tree, callee_id)?;
    if !path_is_regexp_constructor(
        path_segments.as_slice(),
        regexp_name,
        global_qualifier_names,
    ) {
        return None;
    }
    let first_argument_id = *arguments.first()?;

    // update a static flags argument when present
    if arguments.len() >= 2 {
        let second_argument = ctx.tree.get(arguments[1]);
        let ast::Argument::Positional {
            value: flags_expression_id,
            ..
        } = second_argument
        else {
            return None;
        };
        let flags_expression = ctx.tree.get(*flags_expression_id);
        if !matches!(
            flags_expression,
            Expression::ScalarLiteral(ScalarLiteral::String(_))
        ) {
            return None;
        }

        // append the unicode flag to the existing flags literal
        let flags_span = ctx.tree.get_span(*flags_expression_id);
        let flags_text = ctx.get_span_text(flags_span);
        let replacement_flags = append_flag_to_string_literal(flags_text, 'u')?;
        let edits = ctx
            .edit_builder()
            .replace(flags_span, replacement_flags)
            .into_edits();
        return Some(LintFix::safe("Add unicode regex flag").with_edits(edits));
    }

    // add a missing flags argument
    let first_argument_span = ctx.tree.get_span(first_argument_id);
    let edits = ctx
        .edit_builder()
        .insert(first_argument_span.end, ", \"u\"")
        .into_edits();
    Some(LintFix::safe("Add unicode regex flag").with_edits(edits))
}

/// Append one flag character to a quoted string literal source text.
fn append_flag_to_string_literal(text: &str, flag: char) -> Option<String> {
    let first = text.chars().next()?;
    let last = text.chars().last()?;
    if (first != '\'' && first != '"') || first != last || text.len() < 2 {
        return None;
    }

    // keep the original quote style when appending the new flag
    let inner = &text[1..text.len() - 1];
    Some(format!("{first}{inner}{flag}{last}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_regex_without_unicode_flag() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_detects_regex_without_unicode_flag.ds",
            r#"
let re = /foo/
"#,
        );
        test.result(result).assert_lint("require-unicode-regexp");
    }

    #[test]
    fn test_fix_adds_unicode_flag_without_existing_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_fix_adds_unicode_flag_without_existing_flags.ds",
            r#"
let re = /foo/
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = /foo/u;
"#,
            );
    }

    #[test]
    fn test_detects_regex_with_other_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_detects_regex_with_other_flags.ds",
            r#"
let re = /foo/gi
"#,
        );
        test.result(result).assert_lint("require-unicode-regexp");
    }

    #[test]
    fn test_fix_adds_unicode_flag_with_existing_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_fix_adds_unicode_flag_with_existing_flags.ds",
            r#"
let re = /foo/gi
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = /foo/giu;
"#,
            );
    }

    #[test]
    fn test_mutation_fix_adds_unicode_flag_with_single_existing_flag() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_mutation_fix_adds_unicode_flag_with_single_existing_flag.ds",
            r#"
let re = /foo/g
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = /foo/gu;
"#,
            );
    }

    #[test]
    fn test_allows_regex_with_u_flag() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_allows_regex_with_u_flag.ds",
            r#"
let re = /foo/u
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }

    #[test]
    fn test_allows_regex_with_u_and_other_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_allows_regex_with_u_and_other_flags.ds",
            r#"
let re = /foo/giu
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }

    #[test]
    fn test_allows_regex_with_v_flag() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_allows_regex_with_v_flag.ds",
            r#"
let re = /foo/v
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }

    #[test]
    fn test_fix_adds_unicode_flag_to_regexp_constructor_without_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_fix_adds_unicode_flag_to_regexp_constructor_without_flags.ds",
            r#"
let re = RegExp("foo")
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = RegExp("foo", "u");
"#,
            );
    }

    #[test]
    fn test_fix_adds_unicode_flag_to_new_regexp_constructor() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_fix_adds_unicode_flag_to_new_regexp_constructor.ds",
            r#"
let re = new RegExp("foo", "gi")
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = new RegExp("foo", "giu");
"#,
            );
    }

    #[test]
    fn test_allows_regexp_constructor_with_dynamic_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_allows_regexp_constructor_with_dynamic_flags.ds",
            r#"
const flags = "gi";
let re = RegExp("foo", flags);
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }

    #[test]
    fn test_fix_adds_unicode_flag_to_global_this_regexp_constructor() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_fix_adds_unicode_flag_to_global_this_regexp_constructor.ds",
            r#"
let re = globalThis.RegExp("foo");
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = globalThis.RegExp("foo", "u");
"#,
            );
    }
}
