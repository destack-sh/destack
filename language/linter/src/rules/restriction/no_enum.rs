use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow TypeScript enums.
    ///
    /// TypeScript enums have unusual runtime behavior and can lead to larger
    /// bundle sizes. Consider using union types or const objects instead.
    #[lint(
        id = "no-enum",
        code = "LR013",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoEnum,
    "Disallow TypeScript enums"
}

impl LintRule for NoEnum {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEnum::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            if matches!(declaration, ast::Declaration::Enum { .. }) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_ENUM.id,
                        NO_ENUM.code,
                        NO_ENUM.category,
                        severity,
                        "enum declaration",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use union types or const objects instead"),
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
        let test = TestProgram::for_rule_without_builtins(NoEnum);
        let result = test.lint_ast(
            "test.ts",
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
        let test = TestProgram::for_rule_without_builtins(NoEnum);
        let result = test.lint_ast(
            "test.ts",
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
        let test = TestProgram::for_rule_without_builtins(NoEnum);
        let result = test.lint_ast("test.ts", r#"type Color = "red" | "green" | "blue";"#);
        test.result(result).assert_no_lint("no-enum");
    }

    #[test]
    fn test_allows_const_object() {
        let test = TestProgram::for_rule_without_builtins(NoEnum);
        let result = test.lint_ast(
            "test.ts",
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
}
