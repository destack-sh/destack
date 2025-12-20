use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary escape characters in strings.
    ///
    /// Some escape sequences like `\a` are unnecessary in strings because
    /// the character doesn't need escaping.
    #[lint(
        id = "no-useless-escape",
        code = "LU007",
        category = Suspicious,
        level = Ast,
        fixable = No,
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

                let escape_char = raw.chars().nth(char_pos + 1).unwrap_or('?');
                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_ESCAPE.id,
                        NO_USELESS_ESCAPE.code,
                        NO_USELESS_ESCAPE.category,
                        severity,
                        format!("unnecessary escape character: \\{escape_char}"),
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("this escape is unnecessary"),
                );
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
        let test = TestProgram::for_rule_without_builtins(NoUselessEscape);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hel\lo"
"#,
        );
        test.result(result).assert_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_valid_escapes() {
        let test = TestProgram::for_rule_without_builtins(NoUselessEscape);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hello\nworld"
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_quote_escape() {
        let test = TestProgram::for_rule_without_builtins(NoUselessEscape);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "say \"hello\""
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }

    #[test]
    fn test_allows_backslash_escape() {
        let test = TestProgram::for_rule_without_builtins(NoUselessEscape);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "path\\to\\file"
"#,
        );
        test.result(result).assert_no_lint("no-useless-escape");
    }
}
