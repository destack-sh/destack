use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow misleading characters in regex character classes.
    ///
    /// Some Unicode characters combine with adjacent characters, making them
    /// look like a single character when they're actually multiple. Using these
    /// in regex character classes like `[ñ]` may not match what you expect.
    #[lint(
        id = "no-misleading-character-class",
        code = "LU022",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoMisleadingCharacterClass,
    "Disallow misleading regex character classes"
}

impl LintRule for NoMisleadingCharacterClass {
    fn meta(&self) -> &'static LintMeta {
        NoMisleadingCharacterClass::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::RegexString { content, .. }) =
                expr
            else {
                continue;
            };

            let Some(problem) = ctx.regex_misleading_character_class(*content) else {
                continue;
            };
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintReport::new(
                    NO_MISLEADING_CHARACTER_CLASS.id,
                    NO_MISLEADING_CHARACTER_CLASS.code,
                    NO_MISLEADING_CHARACTER_CLASS.category,
                    severity,
                    format!("misleading character in regex character class: {problem}"),
                    ctx.tree.get_span(node_id),
                )
                .label("this character class may not match as expected"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_combining_character() {
        let test = TestProgram::for_rule_without_prelude(NoMisleadingCharacterClass);
        // ñ as n + combining tilde
        let result = test.lint_ast(
            "no_misleading_character_class/test_detects_combining_character.ds",
            "/[n\u{0303}]/",
        );
        test.result(result)
            .assert_lint("no-misleading-character-class");
    }

    #[test]
    fn test_allows_simple_character_class() {
        let test = TestProgram::for_rule_without_prelude(NoMisleadingCharacterClass);
        let result = test.lint_ast(
            "no_misleading_character_class/test_allows_simple_character_class.ds",
            r#"
const re = /[abc]/
"#,
        );
        test.result(result)
            .assert_no_lint("no-misleading-character-class");
    }

    #[test]
    fn test_allows_regex_without_character_class() {
        let test = TestProgram::for_rule_without_prelude(NoMisleadingCharacterClass);
        let result = test.lint_ast(
            "no_misleading_character_class/test_allows_regex_without_character_class.ds",
            r#"
const re = /hello/
"#,
        );
        test.result(result)
            .assert_no_lint("no-misleading-character-class");
    }

    #[test]
    fn test_allows_escaped_bracket() {
        let test = TestProgram::for_rule_without_prelude(NoMisleadingCharacterClass);
        let result = test.lint_ast(
            "no_misleading_character_class/test_allows_escaped_bracket.ds",
            r#"
const re = /\[abc\]/
"#,
        );
        test.result(result)
            .assert_no_lint("no-misleading-character-class");
    }
}
