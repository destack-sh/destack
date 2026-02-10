use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary escape characters in strings.
    ///
    /// Some escape sequences like `\a` are unnecessary in strings because
    /// the character doesn't need escaping.
    #[lint(
        id = "no-useless-escape",
        code = "LU039",
        category = Suspicious,
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessEscape::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let file = ctx.program.files.get(ctx.module.file_id);
        let source = file.text();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_)) = expr else {
                continue;
            };

            let span = ctx.tree.get_span(node_id);
            let start = span.start as usize;
            let end = span.end as usize;

            if start >= source.len() || end > source.len() {
                continue;
            }

            let raw = &source[start..end];

            // check for useless escapes in the raw string
            if let Some(char_pos) = find_useless_escape(raw) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let escape_char = raw[char_pos + 1..].chars().next().unwrap_or('?');
                let mut diagnostic = LintDiagnostic::new(
                    NO_USELESS_ESCAPE.id,
                    NO_USELESS_ESCAPE.code,
                    NO_USELESS_ESCAPE.category,
                    severity,
                    format!("unnecessary escape character: \\{escape_char}"),
                    ctx.module.file_id,
                    span,
                )
                .with_label("this escape is unnecessary");

                // remove only the useless backslash
                let absolute_start = span.start + char_pos as u32;
                let backslash_span = Span::new(span.file, absolute_start, absolute_start + 1);
                let edits = ctx.edit_builder().delete(backslash_span).into_edits();
                let fix = LintFix::safe("Remove unnecessary escape backslash").with_edits(edits);
                diagnostic = diagnostic.with_fix(fix);

                ctx.report(diagnostic);
            }
        }
    }
}

/// Characters that are valid escape sequences in strings.
const VALID_ESCAPES: &[char] = &[
    'n', 'r', 't', 'b', 'f', 'v', '0', '\\', '\'', '"', '`', 'x', 'u', '\n', '\r',
];

/// Find a useless escape in a string literal (returns position of backslash).
fn find_useless_escape(raw: &str) -> Option<usize> {
    let mut chars = raw.char_indices().peekable();

    // determine quote character (first char)
    let quote = chars.next()?.1;
    if !matches!(quote, '"' | '\'' | '`') {
        return None;
    }

    while let Some((pos, c)) = chars.next() {
        if c == '\\'
            && let Some((_, next)) = chars.peek()
        {
            // the quote character used is always valid to escape
            if *next != quote && !VALID_ESCAPES.contains(next) {
                return Some(pos);
            }
            chars.next(); // consume the escaped character
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_useless_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
            "no_useless_escape/test_fix_removes_useless_escape.ds",
            r#"
const x = "hel\lo"
"#,
        );
        test.result(result)
            .assert_lint("no-useless-escape")
            .assert_safe_fixed(
                r#"
const x = "hello";
"#,
            );
    }

    #[test]
    fn test_fix_preserves_valid_escapes_and_removes_only_useless_escape() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint_ast(
            "no_useless_escape/test_fix_preserves_valid_escapes_and_removes_only_useless_escape.ds",
            r#"
const x = "hel\lo\n"
"#,
        );
        test.result(result)
            .assert_lint("no-useless-escape")
            .assert_safe_fixed(
                r#"
const x = "hello\n";
"#,
            );
    }

    #[test]
    fn test_mutation_detects_useless_escape_in_single_quote() {
        let test = TestProgram::for_rule_without_prelude(NoUselessEscape);
        let result = test.lint_ast(
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
        let result = test.lint_ast(
            "no_useless_escape/test_mutation_detects_useless_escape_in_double_quote.ds",
            r#"
const x = "ab\cd"
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }
}
