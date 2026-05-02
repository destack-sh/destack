use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow TypeScript enums.
    ///
    /// TypeScript enums have semantics that differ from plain union types and
    /// object literals, and regular enums can also increase emitted runtime
    /// surface. Consider using union types or const objects instead.
    #[lint(
        id = "no-enum",
        code = "LR011",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoEnum,
    "Disallow TypeScript enums"
}

impl LintRule for NoEnum {
    fn meta(&self) -> &'static LintMeta {
        NoEnum::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate declarations
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            if matches!(declaration, ast::Declaration::Enum { .. }) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintReport::new(
                        NO_ENUM.id,
                        NO_ENUM.code,
                        NO_ENUM.category,
                        severity,
                        "enum declaration",
                        ctx.tree.get_span(node_id),
                    )
                    .label("use union types or const objects instead"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_enum() {
        let test = TestProgram::for_rule_without_prelude(NoEnum);
        let result = test.lint_ast(
            "no_enum/test_detects_enum.ts",
            r#"
enum Color {
    Red,
    Green,
    Blue
}
"#,
        );
        test.result(result).assert_lint("no-enum");
    }

    #[test]
    fn test_detects_const_enum() {
        let test = TestProgram::for_rule_without_prelude(NoEnum);
        let result = test.lint_ast(
            "no_enum/test_detects_const_enum.ts",
            r#"
const enum Direction {
    Up,
    Down
}
"#,
        );
        test.result(result).assert_lint("no-enum");
    }

    #[test]
    fn test_allows_union_type() {
        let test = TestProgram::for_rule_without_prelude(NoEnum);
        let result = test.lint_ast(
            "no_enum/test_allows_union_type.ts",
            r#"type Color = "red" | "green" | "blue";"#,
        );
        test.result(result).assert_no_lint("no-enum");
    }

    #[test]
    fn test_allows_const_object() {
        let test = TestProgram::for_rule_without_prelude(NoEnum);
        let result = test.lint_ast(
            "no_enum/test_allows_const_object.ts",
            r#"
const Color = {
    Red: "red",
    Green: "green",
    Blue: "blue"
} as const;
"#,
        );
        test.result(result).assert_no_lint("no-enum");
    }

    #[test]
    fn test_skips_declaration_file_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoEnum);
        let result = test.lint_ast(
            "no_enum/test_skips_declaration_file_by_default.d.ts",
            r#"
declare enum Color {
    Red,
    Green
}
"#,
        );
        test.result(result).assert_no_lint("no-enum");
    }

    #[test]
    fn test_includes_declaration_file_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoEnum).with_options(|options| {
            options.include_declaration_files = true;
        });
        let result = test.lint_ast(
            "no_enum/test_includes_declaration_file_when_enabled.d.ts",
            r#"
declare enum Color {
    Red,
    Green
}
"#,
        );
        test.result(result).assert_lint("no-enum");
    }

    #[test]
    fn test_detects_ambient_enum_in_source_file() {
        let test = TestProgram::for_rule_without_prelude(NoEnum);
        let result = test.lint_ast(
            "no_enum/test_detects_ambient_enum_in_source_file.ts",
            r#"
declare enum Color {
    Red,
    Green
}
"#,
        );
        test.result(result).assert_lint("no-enum");
    }
}
