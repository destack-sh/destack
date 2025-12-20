use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow class declarations.
    ///
    /// In Destack, structs are preferred over classes for data-oriented design.
    /// Classes encourage inheritance patterns that can lead to complex hierarchies.
    /// Use structs with interface implementations instead.
    #[lint(
        id = "no-class",
        code = "LR006",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoClass,
    "Disallow class declarations"
}

impl LintRule for NoClass {
    fn meta(&self) -> &'static crate::LintMeta {
        NoClass::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            if !matches!(declaration, Declaration::Class { .. }) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_CLASS.id,
                    NO_CLASS.code,
                    NO_CLASS.category,
                    severity,
                    "class declaration is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use struct instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_class() {
        let test = TestProgram::for_rule_without_builtins(NoClass);
        let result = test.lint_ast(
            "test.ts",
            r#"
class MyClass {
    foo() {}
}
"#,
        );
        test.result(result).assert_lint("no-class");
    }

    #[test]
    fn test_detects_exported_class() {
        let test = TestProgram::for_rule_without_builtins(NoClass);
        let result = test.lint_ast(
            "test.ts",
            r#"
export class MyClass {
    foo() {}
}
"#,
        );
        test.result(result).assert_lint("no-class");
    }

    #[test]
    fn test_detects_abstract_class() {
        let test = TestProgram::for_rule_without_builtins(NoClass);
        let result = test.lint_ast(
            "test.ts",
            r#"
abstract class BaseClass {
    abstract foo(): void;
}
"#,
        );
        test.result(result).assert_lint("no-class");
    }

    #[test]
    fn test_allows_struct() {
        let test = TestProgram::for_rule_without_builtins(NoClass);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct MyStruct {
    x: int32;
}
"#,
        );
        test.result(result).assert_no_lint("no-class");
    }

    #[test]
    fn test_allows_interface() {
        let test = TestProgram::for_rule_without_builtins(NoClass);
        let result = test.lint_ast(
            "test.ts",
            r#"
interface MyInterface {
    foo(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-class");
    }

    #[test]
    fn test_allows_function() {
        let test = TestProgram::for_rule_without_builtins(NoClass);
        let result = test.lint_ast(
            "test.ts",
            r#"
function myFunction() {}
"#,
        );
        test.result(result).assert_no_lint("no-class");
    }
}
