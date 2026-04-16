use crate::LintMeta;
use std::collections::HashMap;

use destack_ast as ast;
use destack_core::StringId;
use destack_source::LabeledSpan;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_statement_ancestor;
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

const MIN_DUPLICATE_STRING_LENGTH: usize = 10;
const IGNORED_DUPLICATE_STRINGS: [&str; 1] = ["application/json"];

declare_lint! {
    /// Disallow duplicate string literals.
    ///
    /// When the same string literal is used multiple times, it's often better
    /// to extract it to a constant. This reduces duplication, makes refactoring
    /// easier, and can improve performance through string interning.
    #[lint(
        id = "no-duplicate-string",
        code = "LY016",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoDuplicateString,
    "Disallow duplicate string literals"
}

impl LintRule for NoDuplicateString {
    fn meta(&self) -> &'static LintMeta {
        NoDuplicateString::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // collect all string literals with their locations
        let max_occurrences = ctx.options.complexity.max_duplicate_string_occurrences;
        let mut string_occurrences: HashMap<StringId, Vec<ast::LocalNodeId<ast::Expression>>> =
            HashMap::new();
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(string_id)) =
                expression
            {
                let string_value = ctx.strings.get(*string_id);
                if !string_value_is_reportable(ctx, node_id, string_value.as_ref()) {
                    continue;
                }

                string_occurrences
                    .entry(*string_id)
                    .or_default()
                    .push(node_id);
            }
        }

        // report strings that appear too many times
        for (string_id, occurrences) in string_occurrences {
            if occurrences.len() > max_occurrences {
                // report on the first occurrence
                let first_occurrence = occurrences[0];
                let severity = ctx.get_effective_severity(meta, first_occurrence);
                if !severity.is_enabled() {
                    continue;
                }

                let string_value = ctx.strings.get(string_id);
                let display_value = if string_value.chars().count() > 30 {
                    let truncated = string_value.chars().take(27).collect::<String>();
                    format!("\"{truncated}...\"")
                } else {
                    format!("\"{}\"", &*string_value)
                };

                let mut lint = LintDiagnostic::new(
                    NO_DUPLICATE_STRING.id,
                    NO_DUPLICATE_STRING.code,
                    NO_DUPLICATE_STRING.category,
                    severity,
                    format!(
                        "string {display_value} is repeated {} times (max {max_occurrences})",
                        occurrences.len()
                    ),
                    ctx.module.file_id,
                    ctx.tree.get_span(first_occurrence),
                )
                .with_label("consider extracting to a constant");
                for occurrence in occurrences.iter().skip(1) {
                    lint = lint.with_secondary(LabeledSpan::new(
                        ctx.tree.get_span(*occurrence),
                        "consider extracting to a constant",
                    ));
                }
                ctx.report(lint);
            }
        }
    }
}

/// Return true when one string literal should be considered for duplication checks.
fn string_value_is_reportable(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    string_value: &str,
) -> bool {
    let trimmed_value = string_value.trim();
    if trimmed_value.len() < MIN_DUPLICATE_STRING_LENGTH {
        return false;
    }

    if IGNORED_DUPLICATE_STRINGS.contains(&trimmed_value) {
        return false;
    }

    if expression_statement_ancestor(ctx.tree, ctx.parents, expression_id).is_some() {
        return false;
    }

    trimmed_value
        .chars()
        .any(|character| !character.is_ascii_alphanumeric() && character != '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_strings() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateString)
            .with_options(|options| options.complexity.max_duplicate_string_occurrences = 2);
        let result = test.lint_ast(
            "no_duplicate_string/test_detects_duplicate_strings.ds",
            r#"
let a = "hello world";
let b = "hello world";
let c = "hello world";
"#,
        );
        test.result(result).assert_lint("no-duplicate-string");
    }

    #[test]
    fn test_allows_few_occurrences() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateString)
            .with_options(|options| options.complexity.max_duplicate_string_occurrences = 3);
        let result = test.lint_ast(
            "no_duplicate_string/test_allows_few_occurrences.ds",
            r#"
let a = "hello world";
let b = "hello world";
let c = "hello world";
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-string");
    }

    #[test]
    fn test_ignores_short_strings() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateString)
            .with_options(|options| options.complexity.max_duplicate_string_occurrences = 1);
        let result = test.lint_ast(
            "no_duplicate_string/test_ignores_short_strings.ds",
            r#"
let a = "";
let b = "";
let c = "";
let d = "a";
let e = "a";
let f = "a";
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-string");
    }

    #[test]
    fn test_ignores_string_without_separators() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateString)
            .with_options(|options| options.complexity.max_duplicate_string_occurrences = 1);
        let result = test.lint_ast(
            "no_duplicate_string/test_ignores_string_without_separators.ds",
            r#"
let a = "helloworld";
let b = "helloworld";
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-string");
    }

    #[test]
    fn test_ignores_application_json_literal() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateString)
            .with_options(|options| options.complexity.max_duplicate_string_occurrences = 1);
        let result = test.lint_ast(
            "no_duplicate_string/test_ignores_application_json_literal.ds",
            r#"
let a = "application/json";
let b = "application/json";
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-string");
    }

    #[test]
    fn test_ignores_standalone_expression_statements() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateString)
            .with_options(|options| options.complexity.max_duplicate_string_occurrences = 1);
        let result = test.lint_ast(
            "no_duplicate_string/test_ignores_standalone_expression_statements.ds",
            r#"
"hello world";
"hello world";
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-string");
    }

    #[test]
    fn test_counts_unique_strings_separately() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateString)
            .with_options(|options| options.complexity.max_duplicate_string_occurrences = 2);
        let result = test.lint_ast(
            "no_duplicate_string/test_counts_unique_strings_separately.ds",
            r#"
let a = "hello";
let b = "hello";
let c = "world";
let d = "world";
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-string");
    }

    #[test]
    fn test_handles_unicode_display_truncation() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateString)
            .with_options(|options| options.complexity.max_duplicate_string_occurrences = 1);
        let result = test.lint_ast(
            "no_duplicate_string/test_handles_unicode_display_truncation.ds",
            r#"
let a = "😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀";
let b = "😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀";
"#,
        );
        test.result(result).assert_lint("no-duplicate-string");
    }
}
