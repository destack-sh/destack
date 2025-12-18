use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow control characters in regular expressions.
    ///
    /// Control characters (ASCII codes 0x00-0x1F) are rarely useful in
    /// regular expressions and are often the result of a typo. They can
    /// also cause unexpected behavior.
    #[lint(
        id = "no-control-regex",
        code = "LC017",
        category = Correctness,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoControlRegex,
    "Disallow control characters in regex"
}

impl LintRule for NoControlRegex {
    fn meta(&self) -> &'static crate::LintMeta {
        NoControlRegex::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check regex literals
            let Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) = expression
            else {
                continue;
            };

            let regex_content = ctx.strings.get(*content);
            let regex_str = regex_content.as_ref();

            // check for control characters in the regex
            if let Some(control_char) = find_control_character(regex_str) {
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
}

/// find the first control character in a string
fn find_control_character(s: &str) -> Option<char> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // skip escape sequences that intentionally represent control characters
        if c == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            match next {
                // valid escape sequences for control characters
                'c' => {
                    // \cX control character escape, skip 3 chars
                    i += 3;
                    continue;
                }
                'x' => {
                    // \xHH hex escape, skip 4 chars
                    i += 4;
                    continue;
                }
                'u' => {
                    // \uHHHH unicode escape, skip 6 chars
                    i += 6;
                    continue;
                }
                '0' => {
                    // \0 null, skip 2 chars
                    i += 2;
                    continue;
                }
                'n' | 'r' | 't' | 'f' | 'v' => {
                    // common escape sequences, skip 2 chars
                    i += 2;
                    continue;
                }
                _ => {
                    // other escape, skip 2 chars
                    i += 2;
                    continue;
                }
            }
        }

        // check for raw control characters (0x00-0x1F except common ones)
        if c.is_ascii_control() && c != '\t' && c != '\n' && c != '\r' {
            return Some(c);
        }

        i += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_control_char_in_regex() {
        let test = TestProgram::for_rule(NoControlRegex);
        // use a hex escape to embed a control character
        let result = test.lint_ast("test.ds", "/\x01/");
        test.result(result).assert_lint("no-control-regex");
    }

    #[test]
    fn test_allows_normal_regex() {
        let test = TestProgram::for_rule(NoControlRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /abc/
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }

    #[test]
    fn test_allows_escaped_control_sequences() {
        let test = TestProgram::for_rule(NoControlRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /\n\t\r/
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }

    #[test]
    fn test_allows_hex_escapes() {
        let test = TestProgram::for_rule(NoControlRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /\x00\x1F/
"#,
        );
        test.result(result).assert_no_lint("no-control-regex");
    }
}
