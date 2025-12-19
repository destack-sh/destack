use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow misleading characters in regex character classes.
    ///
    /// Some Unicode characters combine with adjacent characters, making them
    /// look like a single character when they're actually multiple. Using these
    /// in regex character classes like `[ñ]` may not match what you expect.
    #[lint(
        id = "no-misleading-character-class",
        code = "LU013",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoMisleadingCharacterClass,
    "Disallow misleading regex character classes"
}

impl LintRule for NoMisleadingCharacterClass {
    fn meta(&self) -> &'static crate::LintMeta {
        NoMisleadingCharacterClass::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::RegexString { content, .. }) =
                expr
            else {
                continue;
            };

            let regex_content = ctx.strings.get(*content);
            let regex_str = regex_content.as_ref();

            if let Some(problem) = find_misleading_character_class(regex_str) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_MISLEADING_CHARACTER_CLASS.id,
                        NO_MISLEADING_CHARACTER_CLASS.code,
                        NO_MISLEADING_CHARACTER_CLASS.category,
                        severity,
                        format!("misleading character in regex character class: {problem}"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this character class may not match as expected"),
                );
            }
        }
    }
}

/// Find misleading characters in a regex character class.
/// Returns a description of the problem if found.
fn find_misleading_character_class(regex: &str) -> Option<&'static str> {
    let mut in_class = false;
    let mut chars = regex.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // skip escaped character
                chars.next();
            }
            '[' if !in_class => {
                in_class = true;
            }
            ']' if in_class => {
                in_class = false;
            }
            _ if in_class => {
                // check for combining characters
                if is_combining_mark(c) {
                    return Some("combining character");
                }

                // surrogate pairs are handled by Rust's char type (they can't exist as chars)

                // check for regional indicator symbols (emoji flags)
                if c >= '\u{1F1E6}' && c <= '\u{1F1FF}' {
                    return Some("regional indicator symbol");
                }

                // check for zero-width characters
                if is_zero_width(c) {
                    return Some("zero-width character");
                }
            }
            _ => {}
        }
    }

    None
}

/// Check if a character is a combining mark.
fn is_combining_mark(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}' |  // combining diacritical marks
        '\u{1AB0}'..='\u{1AFF}' |  // combining diacritical marks extended
        '\u{1DC0}'..='\u{1DFF}' |  // combining diacritical marks supplement
        '\u{20D0}'..='\u{20FF}' |  // combining diacritical marks for symbols
        '\u{FE20}'..='\u{FE2F}'    // combining half marks
    )
}

/// Check if a character is a zero-width character.
fn is_zero_width(c: char) -> bool {
    matches!(
        c,
        '\u{200B}' |  // zero-width space
        '\u{200C}' |  // zero-width non-joiner
        '\u{200D}' |  // zero-width joiner
        '\u{FEFF}' // zero-width no-break space (BOM)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_combining_character() {
        let test = TestProgram::for_rule(NoMisleadingCharacterClass);
        // ñ as n + combining tilde
        let result = test.lint_ast("test.ds", "/[n\u{0303}]/");
        test.result(result)
            .assert_lint("no-misleading-character-class");
    }

    #[test]
    fn test_allows_simple_character_class() {
        let test = TestProgram::for_rule(NoMisleadingCharacterClass);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /[abc]/
"#,
        );
        test.result(result)
            .assert_no_lint("no-misleading-character-class");
    }

    #[test]
    fn test_allows_regex_without_character_class() {
        let test = TestProgram::for_rule(NoMisleadingCharacterClass);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /hello/
"#,
        );
        test.result(result)
            .assert_no_lint("no-misleading-character-class");
    }

    #[test]
    fn test_allows_escaped_bracket() {
        let test = TestProgram::for_rule(NoMisleadingCharacterClass);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /\[abc\]/
"#,
        );
        test.result(result)
            .assert_no_lint("no-misleading-character-class");
    }
}
