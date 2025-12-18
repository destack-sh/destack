use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Warn on exclusive ranges that are likely meant to be inclusive.
    ///
    /// This lint detects exclusive ranges where the end boundary suggests
    /// the author may have intended an inclusive range. Common cases include:
    ///
    /// - Character ranges like `'a'..'z'` (excludes 'z', likely meant `'a'..='z'`)
    /// - Ranges ending in subtraction like `0..n - 1` (might mean `0..n` or `0..=n - 1`)
    ///
    /// ```
    /// // Suspicious
    /// for c of 'a'..'z' { }       // excludes 'z'
    /// for i of 0..len - 1 { }     // is the -1 intentional?
    ///
    /// // Clear intent
    /// for c of 'a'..='z' { }      // includes 'z'
    /// for i of 0..len { }         // excludes len (standard pattern)
    /// for i of 0..=len - 1 { }    // explicitly includes len - 1
    /// ```
    #[lint(
        id = "no-incomplete-range",
        code = "LU026",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoIncompleteRange,
    "Warn on exclusive ranges that may be incomplete"
}

impl LintRule for NoIncompleteRange {
    fn meta(&self) -> &'static crate::LintMeta {
        NoIncompleteRange::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } = expression
            else {
                continue;
            };

            // only check exclusive ranges
            if *is_inclusive {
                continue;
            }

            let start_expression = ctx.tree.get(*start);
            let end_expression = ctx.tree.get(*end);

            // check for character range patterns like 'a'..'z'
            if let Some((start_char, end_char)) = get_char_range(start_expression, end_expression)
                && is_suspicious_char_range(start_char, end_char)
            {
                ctx.report(
                    LintDiagnostic::new(
                        NO_INCOMPLETE_RANGE.id,
                        NO_INCOMPLETE_RANGE.code,
                        NO_INCOMPLETE_RANGE.category,
                        severity,
                        format!(
                            "exclusive range `'{start_char}'..'{end_char}'` excludes `'{end_char}'`",
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label(format!(
                        "use `'{start_char}'..='{end_char}'` to include `'{end_char}'`",
                    )),
                );
                continue;
            }

            // check for subtraction at the end like `0..n - 1`
            if is_end_subtraction(end_expression) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_INCOMPLETE_RANGE.id,
                        NO_INCOMPLETE_RANGE.code,
                        NO_INCOMPLETE_RANGE.category,
                        severity,
                        "exclusive range with subtracted end may be off-by-one",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("consider using inclusive range `..=` or removing the subtraction"),
                );
            }
        }
    }
}

/// Extract character values from a character range.
fn get_char_range(start: &Expression, end: &Expression) -> Option<(char, char)> {
    let start_char = match start {
        Expression::ScalarLiteral(ScalarLiteral::Character(c)) => *c,
        _ => return None,
    };
    let end_char = match end {
        Expression::ScalarLiteral(ScalarLiteral::Character(c)) => *c,
        _ => return None,
    };
    Some((start_char, end_char))
}

/// Check if a character range looks suspicious (likely meant to be inclusive).
fn is_suspicious_char_range(start: char, end: char) -> bool {
    // common alphabet/digit ranges where end is likely meant to be included
    matches!(
        (start, end),
        // lowercase alphabet
        ('a', 'z') |
        // uppercase alphabet
        ('A', 'Z') |
        // digits
        ('0', '9') |
        // common subranges
        ('a', 'f') | ('A', 'F') | // hex digits
        ('a', 'm') | ('n', 'z') | // alphabet halves
        ('A', 'M') | ('N', 'Z')
    )
}

/// Check if the end expression is a subtraction operation.
fn is_end_subtraction(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Binary {
            operator: ast::BinaryOperator::Subtract,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_char_range_a_to_z_detected() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 'a'..'z'
"#,
        );
        test.result(result).assert_lint("no-incomplete-range");
    }

    #[test]
    fn test_char_range_uppercase_detected() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 'A'..'Z'
"#,
        );
        test.result(result).assert_lint("no-incomplete-range");
    }

    #[test]
    fn test_char_range_digits_detected() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = '0'..'9'
"#,
        );
        test.result(result).assert_lint("no-incomplete-range");
    }

    #[test]
    fn test_char_range_hex_detected() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 'a'..'f'
"#,
        );
        test.result(result).assert_lint("no-incomplete-range");
    }

    #[test]
    fn test_subtraction_at_end_detected() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..n - 1
"#,
        );
        test.result(result).assert_lint("no-incomplete-range");
    }

    #[test]
    fn test_inclusive_char_range_allowed() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 'a'..='z'
"#,
        );
        test.result(result).assert_no_lint("no-incomplete-range");
    }

    #[test]
    fn test_inclusive_with_subtraction_allowed() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..=n - 1
"#,
        );
        test.result(result).assert_no_lint("no-incomplete-range");
    }

    #[test]
    fn test_numeric_range_without_subtraction_allowed() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 0..10
"#,
        );
        test.result(result).assert_no_lint("no-incomplete-range");
    }

    #[test]
    fn test_random_char_range_allowed() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
const range = 'b'..'y'
"#,
        );
        test.result(result).assert_no_lint("no-incomplete-range");
    }

    #[test]
    fn test_for_loop_char_range_detected() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    for (const c of 'a'..'z') {
        print(c)
    }
}
"#,
        );
        test.result(result).assert_lint("no-incomplete-range");
    }

    #[test]
    fn test_for_loop_length_minus_one_detected() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(arr: int[]) {
    for (const i of 0..arr.length - 1) {
        print(arr[i])
    }
}
"#,
        );
        test.result(result).assert_lint("no-incomplete-range");
    }

    #[test]
    fn test_standard_length_pattern_allowed() {
        let test = TestProgram::for_rule(NoIncompleteRange);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(arr: int[]) {
    for (const i of 0..arr.length) {
        print(arr[i])
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-incomplete-range");
    }
}
