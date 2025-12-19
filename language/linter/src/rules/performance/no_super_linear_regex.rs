use destack_ast as ast;
use destack_workspace::LintSeverity;
use regex_syntax::Parser;
use regex_syntax::hir::{Hir, HirKind};

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow regular expressions with super-linear worst-case complexity.
    ///
    /// Certain regex patterns can exhibit catastrophic backtracking, where
    /// the matching time grows exponentially or polynomially with input length.
    /// This commonly occurs with nested quantifiers like `(a+)+` or `(a*)*`.
    ///
    /// ## Bad
    /// ```
    /// let re = /(a+)+/        // exponential backtracking
    /// let re = /(a*)*b/       // exponential backtracking
    /// let re = /(a+)*$/       // exponential backtracking on non-matching input
    /// let re = /(.*a){10}/    // polynomial complexity
    /// ```
    ///
    /// ## Good
    /// ```
    /// let re = /a+/           // linear
    /// let re = /(ab)+/        // linear
    /// let re = /(?:a+)b/      // linear with atomic/possessive if supported
    /// ```
    #[lint(
        id = "no-super-linear-regex",
        code = "LP004",
        category = Performance,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoSuperLinearRegex,
    "Disallow regex with potential catastrophic backtracking"
}

impl LintRule for NoSuperLinearRegex {
    fn meta(&self) -> &'static crate::LintMeta {
        NoSuperLinearRegex::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::RegexString { content, .. }) =
                expression
            else {
                continue;
            };

            let regex_content = ctx.strings.get(*content);
            let regex_str = regex_content.as_ref();

            // parse the regex into HIR for analysis
            let Ok(hir) = Parser::new().parse(regex_str) else {
                // invalid regex: handled by no-invalid-regexp
                continue;
            };

            if let Some(problem) = find_super_linear_pattern(&hir) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_SUPER_LINEAR_REGEX.id,
                        NO_SUPER_LINEAR_REGEX.code,
                        NO_SUPER_LINEAR_REGEX.category,
                        severity,
                        problem,
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this pattern may cause catastrophic backtracking"),
                );
            }
        }
    }
}

/// Find super-linear (exponential/polynomial) patterns in a regex HIR.
/// Returns a description of the problem if found.
fn find_super_linear_pattern(hir: &Hir) -> Option<String> {
    check_nested_quantifiers(hir, false)
}

/// Recursively check for nested quantifiers.
/// `in_unbounded_quantifier` is true when we're inside an unbounded repetition.
fn check_nested_quantifiers(hir: &Hir, in_unbounded_quantifier: bool) -> Option<String> {
    match hir.kind() {
        HirKind::Repetition(rep) => {
            // check if this is a truly unbounded repetition (+ or * or {n,})
            // bounded repetitions like {1,3} don't cause exponential backtracking
            let is_unbounded = rep.max.is_none();

            if in_unbounded_quantifier && is_unbounded {
                // nested unbounded quantifier found
                return Some("nested quantifiers can cause exponential backtracking".to_string());
            }

            // check inside the repetition, marking that we're in an unbounded quantifier
            check_nested_quantifiers(&rep.sub, is_unbounded)
        }

        HirKind::Concat(items) => {
            // check all items in sequence
            for item in items {
                if let Some(problem) = check_nested_quantifiers(item, in_unbounded_quantifier) {
                    return Some(problem);
                }
            }
            None
        }

        HirKind::Alternation(alts) => {
            // check all alternatives
            for alt in alts {
                if let Some(problem) = check_nested_quantifiers(alt, in_unbounded_quantifier) {
                    return Some(problem);
                }
            }

            // also check for overlapping alternatives inside an unbounded quantifier
            if in_unbounded_quantifier && alts.len() >= 2 && check_overlapping_alternatives(alts) {
                return Some(
                    "overlapping alternatives in quantifier can cause exponential backtracking"
                        .to_string(),
                );
            }

            None
        }

        HirKind::Capture(cap) => check_nested_quantifiers(&cap.sub, in_unbounded_quantifier),

        // literals and other terminals don't cause super-linear behavior
        HirKind::Empty | HirKind::Literal(_) | HirKind::Class(_) | HirKind::Look(_) => None,
    }
}

/// Check if alternatives have overlapping prefixes (e.g., `(a|ab)` or `(a|a)`).
/// This is a simplified check that looks for alternatives that both start
/// with the same character class or literal.
fn check_overlapping_alternatives(alts: &[Hir]) -> bool {
    // get the first character possibility for each alternative
    let first_chars: Vec<_> = alts.iter().filter_map(get_first_char_class).collect();

    // check for overlaps between any pair
    for i in 0..first_chars.len() {
        for j in (i + 1)..first_chars.len() {
            if classes_overlap(&first_chars[i], &first_chars[j]) {
                return true;
            }
        }
    }

    false
}

/// Represents a set of characters that can match.
#[derive(Clone)]
enum CharClass {
    /// A specific literal character.
    Literal(char),
    /// Any character (from `.`).
    Any,
    /// A Unicode class.
    Class,
}

/// Get the first character class from an HIR.
fn get_first_char_class(hir: &Hir) -> Option<CharClass> {
    match hir.kind() {
        HirKind::Literal(lit) => {
            // get the first character from the literal
            let s = std::str::from_utf8(&lit.0).ok()?;
            s.chars().next().map(CharClass::Literal)
        }

        HirKind::Class(_) => Some(CharClass::Class),

        HirKind::Concat(items) => {
            // get first from first item
            items.first().and_then(get_first_char_class)
        }

        HirKind::Alternation(alts) => {
            // if any branch can match any character, return Any
            for alt in alts {
                if let Some(CharClass::Any) = get_first_char_class(alt) {
                    return Some(CharClass::Any);
                }
            }
            // otherwise return Class (simplification)
            if !alts.is_empty() {
                Some(CharClass::Class)
            } else {
                None
            }
        }

        HirKind::Capture(cap) => get_first_char_class(&cap.sub),

        HirKind::Repetition(rep) => {
            // if it can match zero times, it doesn't contribute a required first char
            if rep.min == 0 {
                None
            } else {
                get_first_char_class(&rep.sub)
            }
        }

        HirKind::Empty => None,
        HirKind::Look(_) => None,
    }
}

/// Check if two character classes could match the same character.
fn classes_overlap(a: &CharClass, b: &CharClass) -> bool {
    match (a, b) {
        // any overlaps with everything
        (CharClass::Any, _) | (_, CharClass::Any) => true,
        // class overlaps with anything (simplification)
        (CharClass::Class, _) | (_, CharClass::Class) => true,
        // literals overlap if they're the same
        (CharClass::Literal(c1), CharClass::Literal(c2)) => c1 == c2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nested_plus() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(a+)+/
"#,
        );
        test.result(result).assert_lint("no-super-linear-regex");
    }

    #[test]
    fn test_detects_nested_star() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(a*)*b/
"#,
        );
        test.result(result).assert_lint("no-super-linear-regex");
    }

    #[test]
    fn test_detects_star_plus() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(a+)*/
"#,
        );
        test.result(result).assert_lint("no-super-linear-regex");
    }

    #[test]
    fn test_detects_deeply_nested() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /((a+)+)+/
"#,
        );
        test.result(result).assert_lint("no-super-linear-regex");
    }

    #[test]
    fn test_allows_simple_quantifier() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /a+/
"#,
        );
        test.result(result).assert_no_lint("no-super-linear-regex");
    }

    #[test]
    fn test_allows_group_with_quantifier() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(ab)+/
"#,
        );
        test.result(result).assert_no_lint("no-super-linear-regex");
    }

    #[test]
    fn test_allows_bounded_repetition() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(a{1,3}){1,3}/
"#,
        );
        // bounded repetitions don't cause exponential backtracking
        test.result(result).assert_no_lint("no-super-linear-regex");
    }

    #[test]
    fn test_allows_optional_in_quantifier() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(ab?)+/
"#,
        );
        // ? is bounded (0-1), so not super-linear by itself
        test.result(result).assert_no_lint("no-super-linear-regex");
    }

    #[test]
    fn test_allows_simple_regex() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /^[a-z]+@[a-z]+\.[a-z]+$/
"#,
        );
        test.result(result).assert_no_lint("no-super-linear-regex");
    }

    #[test]
    fn test_allows_non_nested_alternation() {
        let test = TestProgram::for_rule(NoSuperLinearRegex);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(foo|bar)+/
"#,
        );
        // non-overlapping alternatives are fine
        test.result(result).assert_no_lint("no-super-linear-regex");
    }
}
