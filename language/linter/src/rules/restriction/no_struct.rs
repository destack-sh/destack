use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow struct declarations.
    ///
    /// Some codebases prefer classes over structs for consistency with
    /// existing TypeScript patterns or when identity semantics are needed.
    /// This rule enforces class-only object definitions.
    #[lint(
        id = "no-struct",
        code = "LR014",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoStruct,
    "Disallow struct declarations"
}

impl LintRule for NoStruct {
    fn meta(&self) -> &'static crate::LintMeta {
        NoStruct::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            if !matches!(declaration, Declaration::Struct { .. }) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_STRUCT.id,
                    NO_STRUCT.code,
                    NO_STRUCT.category,
                    severity,
                    "struct declaration is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use class instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_struct() {
        let test = TestProgram::for_rule_without_builtins(NoStruct);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct Point {
    x: int32,
    y: int32,
}
"#,
        );
        test.result(result).assert_lint("no-struct");
    }

    #[test]
    fn test_detects_exported_struct() {
        let test = TestProgram::for_rule_without_builtins(NoStruct);
        let result = test.lint_ast(
            "test.ds",
            r#"
export struct Point {
    x: int32,
    y: int32,
}
"#,
        );
        test.result(result).assert_lint("no-struct");
    }

    #[test]
    fn test_allows_class() {
        let test = TestProgram::for_rule_without_builtins(NoStruct);
        let result = test.lint_ast(
            "test.ts",
            r#"
class MyClass {
    foo() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-struct");
    }

    #[test]
    fn test_allows_interface() {
        let test = TestProgram::for_rule_without_builtins(NoStruct);
        let result = test.lint_ast(
            "test.ts",
            r#"
interface MyInterface {
    foo(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-struct");
    }

    #[test]
    fn test_allows_type() {
        let test = TestProgram::for_rule_without_builtins(NoStruct);
        let result = test.lint_ast(
            "test.ts",
            r#"
type Point = { x: number, y: number };
"#,
        );
        test.result(result).assert_no_lint("no-struct");
    }
}
