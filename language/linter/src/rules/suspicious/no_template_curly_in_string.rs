use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Disallow template interpolation in regular strings.
    ///
    /// Using `"${x}"` in a regular string instead of a template literal
    /// `` `${x}` `` won't interpolate the variable and is likely a mistake.
    #[lint(
        id = "no-template-curly-in-string",
        code = "LU031",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoTemplateCurlyInString,
    "Disallow template interpolation in regular strings"
}

impl LintRule for NoTemplateCurlyInString {
    fn meta(&self) -> &'static LintMeta {
        NoTemplateCurlyInString::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(string_id)) = expr else {
                continue;
            };

            // skip string literals that do not contain `${` at all
            let string_value = ctx.strings.get(*string_id);
            if !string_value.as_ref().contains("${") {
                continue;
            }

            // detect template interpolation from source text to avoid escaped `${` false positives
            let span = ctx.tree.get_span(node_id);
            let literal_text = ctx.get_span_text(span);
            if !contains_template_interpolation(literal_text) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                NO_TEMPLATE_CURLY_IN_STRING.id,
                NO_TEMPLATE_CURLY_IN_STRING.code,
                NO_TEMPLATE_CURLY_IN_STRING.category,
                severity,
                "template interpolation in regular string",
                ctx.module.file_id,
                span,
            )
            .with_label("use a template literal `...` instead of \"...\"");

            if ctx.compute_fixes
                && let Some(fix) = template_literal_fix(ctx, span, literal_text)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Check if a string contains template interpolation like `${...}`.
fn contains_template_interpolation(s: &str) -> bool {
    let Some((quote, content)) = split_quoted_string_literal(s) else {
        return false;
    };
    if quote == '`' {
        return false;
    }

    let bytes = content.as_bytes();
    for index in 0..bytes.len().saturating_sub(1) {
        if bytes[index] != b'$' || bytes[index + 1] != b'{' {
            continue;
        }

        if is_escaped(content, index) {
            continue;
        }

        let mut depth = 1;
        let mut cursor = index + 2;
        while cursor < bytes.len() {
            let current = bytes[cursor];
            if current == b'{' && !is_escaped(content, cursor) {
                depth += 1;
            } else if current == b'}' && !is_escaped(content, cursor) {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            cursor += 1;
        }
    }

    false
}

/// Return (quote, content) for a quoted string literal.
fn split_quoted_string_literal(source: &str) -> Option<(char, &str)> {
    let quote = source.chars().next()?;
    if !matches!(quote, '"' | '\'' | '`') || !source.ends_with(quote) {
        return None;
    }

    let content_start = quote.len_utf8();
    let content_end = source.len().saturating_sub(quote.len_utf8());
    Some((quote, &source[content_start..content_end]))
}

/// Return true when the byte at index is escaped by an odd number of backslashes.
fn is_escaped(source: &str, index: usize) -> bool {
    let bytes = source.as_bytes();
    if index == 0 || index > bytes.len() {
        return false;
    }

    let mut slash_count = 0;
    let mut cursor = index;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        slash_count += 1;
        cursor -= 1;
    }

    slash_count % 2 == 1
}

/// Build a fix to convert a regular string literal to a template literal.
fn template_literal_fix(
    ctx: &LintAstContext<'_>,
    span: destack_source::Span,
    literal_text: &str,
) -> Option<LintFix> {
    let (quote, content) = split_quoted_string_literal(literal_text)?;
    if quote == '`' || content.contains('`') {
        return None;
    }

    let replacement = format!("`{content}`");
    let edits = ctx.edit_builder().replace(span, replacement).into_edits();
    Some(LintFix::r#unsafe("Convert to a template literal").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_template_in_double_quoted_string() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_detects_template_in_double_quoted_string.ds",
            r#"
const x = "Hello ${name}"
"#,
        );
        test.result(result)
            .assert_lint("no-template-curly-in-string")
            .assert_has_fix("no-template-curly-in-string");
    }

    #[test]
    fn test_detects_template_in_single_quoted_string() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_detects_template_in_single_quoted_string.ds",
            r#"
const x = 'Hello ${name}'
"#,
        );
        test.result(result)
            .assert_lint("no-template-curly-in-string")
            .assert_unsafe_fixed(
                r#"
const x = `Hello ${name}`;
"#,
            );
    }

    #[test]
    fn test_detects_multiple_templates() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_detects_multiple_templates.ds",
            r#"
const x = "${a} + ${b} = ${c}"
"#,
        );
        test.result(result)
            .assert_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_template_literal() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_allows_template_literal.ds",
            r#"
const x = `Hello ${name}`
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_regular_string() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_allows_regular_string.ds",
            r#"
const x = "Hello world"
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_dollar_without_brace() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_allows_dollar_without_brace.ds",
            r#"
const x = "Price: $100"
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_incomplete_template() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_allows_incomplete_template.ds",
            r#"
const x = "${unclosed"
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_escaped_template_marker() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_allows_escaped_template_marker.ds",
            r#"
const x = "\${name}"
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_fix_converts_to_template_literal() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_fix_converts_to_template_literal.ds",
            r#"
const x = "Hello ${name}"
"#,
        );
        test.result(result)
            .assert_lint("no-template-curly-in-string")
            .assert_unsafe_fixed(
                r#"
const x = `Hello ${name}`;
"#,
            );
    }

    #[test]
    fn test_no_fix_when_content_contains_backtick() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_no_fix_when_content_contains_backtick.ds",
            r#"
const x = "value: ${name}, marker: `"
"#,
        );
        test.result(result)
            .assert_lint("no-template-curly-in-string")
            .assert_has_no_fix("no-template-curly-in-string");
    }

    #[test]
    fn test_mutation_detects_nested_braces() {
        let test = TestProgram::for_rule_without_prelude(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "no_template_curly_in_string/test_mutation_detects_nested_braces.ds",
            r#"
const x = "value: ${format({ id: userId })}"
"#,
        );
        test.result(result)
            .assert_lint("no-template-curly-in-string")
            .assert_unsafe_fixed(
                r#"
const x = `value: ${format({ id: userId })}`;
"#,
            );
    }
}
