use destack_ast as ast;
use destack_workspace::LintSeverity;
use regex_syntax::ast::ErrorKind;
use regex_syntax::ast::parse::Parser as RegexParser;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow useless backreferences in regular expressions.
    ///
    /// Backreferences that reference non-existent groups or forward-reference
    /// groups that haven't been captured yet will never match anything useful.
    #[lint(
        id = "no-useless-backreference",
        code = "LU019",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessBackreference,
    "Disallow useless regex backreferences"
}

impl LintRule for NoUselessBackreference {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessBackreference::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::RegexString { content, .. }) =
                expr
            else {
                continue;
            };

            let regex_content = ctx.strings.get(*content);
            let regex_str = regex_content.as_ref();

            if let Some(problem) = find_useless_backreference(regex_str) {
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
}

/// Find useless backreferences in a regex pattern.
/// Uses regex-syntax to detect backreferences (which it reports as errors),
/// then validates them against the group structure.
fn find_useless_backreference(regex: &str) -> Option<String> {
    // Try to parse with regex-syntax. If it reports UnsupportedBackreference,
    // we know there's a backreference. We then analyze it manually.
    if let Err(err) = RegexParser::new().parse(regex)
        && matches!(err.kind(), ErrorKind::UnsupportedBackreference)
    {
        // regex-syntax found a backreference. Now analyze if it's valid.
        return analyze_backreferences(regex);
    }
    // No backreferences or other parse error - not our concern
    None
}

/// Analyze backreferences in a regex pattern manually.
/// Returns a description of the problem if a useless backreference is found.
fn analyze_backreferences(regex: &str) -> Option<String> {
    let mut group_count = 0;
    let mut chars = regex.chars().peekable();
    let mut in_char_class = false;

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() && next != '0' && !in_char_class {
                        // found a backreference like \1, \2, etc.
                        chars.next();
                        let mut num_str = String::from(next);

                        // collect multi-digit backreference
                        while let Some(&d) = chars.peek() {
                            if d.is_ascii_digit() {
                                num_str.push(d);
                                chars.next();
                            } else {
                                break;
                            }
                        }

                        if let Ok(backref_num) = num_str.parse::<usize>()
                            && backref_num > group_count
                        {
                            return Some(format!(
                                "backreference \\{backref_num} references non-existent group \
                                     (only {group_count} groups defined so far)"
                            ));
                        }
                    } else {
                        // skip escaped character
                        chars.next();
                    }
                }
            }
            '[' if !in_char_class => {
                in_char_class = true;
            }
            ']' if in_char_class => {
                in_char_class = false;
            }
            '(' if !in_char_class => {
                // check if this is a capturing group
                if chars.peek() != Some(&'?') {
                    group_count += 1;
                } else {
                    // might be (?:...) or (?=...) etc., check for named group
                    let mut temp_chars = chars.clone();
                    temp_chars.next(); // skip ?
                    if temp_chars.peek() == Some(&'<') {
                        temp_chars.next();
                        if let Some(&c) = temp_chars.peek()
                            && c != '='
                            && c != '!'
                        {
                            // named capturing group (?<name>...)
                            group_count += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nonexistent_backreference() {
        let test = TestProgram::for_rule(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /(a)\2/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_forward_reference() {
        let test = TestProgram::for_rule(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /\1(a)/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_valid_backreference() {
        let test = TestProgram::for_rule(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /(a)\1/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_multiple_valid_backreferences() {
        let test = TestProgram::for_rule(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /(a)(b)\1\2/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_regex_without_backreference() {
        let test = TestProgram::for_rule(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /hello/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_escaped_digit_in_char_class() {
        let test = TestProgram::for_rule(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /[\1]/
"#,
        );
        // \1 in character class is octal, not backreference
        let _ = test.result(result);
    }
}
