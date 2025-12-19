use std::collections::HashMap;

use destack_ast as ast;
use destack_base::StringId;
use destack_source::LabeledSpan;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate string literals.
    ///
    /// When the same string literal is used multiple times, it's often better
    /// to extract it to a constant. This reduces duplication, makes refactoring
    /// easier, and can improve performance through string interning.
    #[lint(
        id = "no-duplicate-string",
        code = "LY020",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoDuplicateString,
    "Disallow duplicate string literals"
}

impl LintRule for NoDuplicateString {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDuplicateString::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // collect all string literals with their locations
        let max_occurrences = ctx.options.max_duplicate_string_occurrences;
        let mut string_occurrences: HashMap<StringId, Vec<ast::LocalNodeId<ast::Expression>>> =
            HashMap::new();
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(string_id)) =
                expression
            {
                // skip very short strings (likely intentional repetition like "", " ", etc.)
                let string_value = ctx.strings.get(*string_id);
                if string_value.len() >= 3 {
                    string_occurrences
                        .entry(*string_id)
                        .or_default()
                        .push(node_id);
                }
            }
        }

        // report strings that appear too many times
        for (string_id, occurrences) in string_occurrences {
            if occurrences.len() > max_occurrences {
                let string_value = ctx.strings.get(string_id);
                let display_value = if string_value.len() > 30 {
                    format!("\"{}...\"", &string_value[..27])
                } else {
                    format!("\"{}\"", &*string_value)
                };

                // report on the first occurrence
                let first_occurrence = occurrences[0];
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_strings() {
        let test = TestProgram::for_rule(NoDuplicateString)
            .with_options(|options| options.max_duplicate_string_occurrences = 2);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(NoDuplicateString)
            .with_options(|options| options.max_duplicate_string_occurrences = 3);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(NoDuplicateString)
            .with_options(|options| options.max_duplicate_string_occurrences = 1);
        let result = test.lint_ast(
            "test.ds",
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
    fn test_counts_unique_strings_separately() {
        let test = TestProgram::for_rule(NoDuplicateString)
            .with_options(|options| options.max_duplicate_string_occurrences = 2);
        let result = test.lint_ast(
            "test.ds",
            r#"
let a = "hello";
let b = "hello";
let c = "world";
let d = "world";
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-string");
    }
}
