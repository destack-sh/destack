use crate::LintMeta;
use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

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
        level = Dir,
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expr = ctx.dir.get(node_id);
            let dir::Expression::ScalarLiteral(dir::ScalarLiteral::RegexString { content, .. }) =
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
                    ctx.dir.get_span(node_id),
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
        let result = test.lint(
            "no_misleading_character_class/test_detects_combining_character.ds",
            "/[n\u{0303}]/",
        );
        test.result(result)
            .assert_lint("no-misleading-character-class");
    }

    #[test]
    fn test_allows_simple_character_class() {
        let test = TestProgram::for_rule_without_prelude(NoMisleadingCharacterClass);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "no_misleading_character_class/test_allows_escaped_bracket.ds",
            r#"
const re = /\[abc\]/
"#,
        );
        test.result(result)
            .assert_no_lint("no-misleading-character-class");
    }
}
