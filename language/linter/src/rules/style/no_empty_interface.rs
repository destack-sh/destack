use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty interface declarations.
    ///
    /// Empty interfaces are often a sign of incomplete code. If the interface
    /// has no members and doesn't extend anything, use `type X = {}` or
    /// `type X = object` instead. If it extends a single type, use a type alias.
    #[lint(
        id = "no-empty-interface",
        code = "LY033",
        category = Style,
        level = Ast
    )]
    pub NoEmptyInterface,
    "Disallow empty interface declarations"
}

impl LintRule for NoEmptyInterface {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEmptyInterface::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let Declaration::Interface {
                members, heritage, ..
            } = declaration
            else {
                continue;
            };

            let extends_count = heritage
                .extends_types
                .as_ref()
                .map(|v| v.len())
                .unwrap_or(0);

            // empty interface with no extends is useless
            if members.is_empty() && extends_count == 0 {
                let span = ctx.tree.get_span(node_id);
                ctx.report(
                    LintDiagnostic::new(
                        NO_EMPTY_INTERFACE.id,
                        NO_EMPTY_INTERFACE.code,
                        NO_EMPTY_INTERFACE.category,
                        severity,
                        "empty interface declaration",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("use `type X = {}` or add members"),
                );
            }
            // interface that only extends one type could be a type alias
            else if members.is_empty() && extends_count == 1 {
                let span = ctx.tree.get_span(node_id);
                ctx.report(
                    LintDiagnostic::new(
                        NO_EMPTY_INTERFACE.id,
                        NO_EMPTY_INTERFACE.code,
                        NO_EMPTY_INTERFACE.category,
                        severity,
                        "interface extends single type without adding members",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("use `type X = Parent` or `newtype X = Parent` instead"),
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
    fn test_detects_empty_interface() {
        let test = TestProgram::for_rule(NoEmptyInterface);
        let result = test.lint_ast(
            "test.ts",
            r#"
interface Empty {}
"#,
        );
        test.result(result).assert_lint("no-empty-interface");
    }

    #[test]
    fn test_detects_single_extends() {
        let test = TestProgram::for_rule(NoEmptyInterface);
        let result = test.lint_ast(
            "test.ts",
            r#"
interface Child extends Parent {}
"#,
        );
        test.result(result).assert_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_interface_with_members() {
        let test = TestProgram::for_rule(NoEmptyInterface);
        let result = test.lint_ast(
            "test.ts",
            r#"
interface Foo {
    bar(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_multiple_extends() {
        let test = TestProgram::for_rule(NoEmptyInterface);
        let result = test.lint_ast(
            "test.ts",
            r#"
interface Combined extends A, B {}
"#,
        );
        test.result(result).assert_no_lint("no-empty-interface");
    }

    #[test]
    fn test_allows_extends_with_members() {
        let test = TestProgram::for_rule(NoEmptyInterface);
        let result = test.lint_ast(
            "test.ts",
            r#"
interface Child extends Parent {
    extra(): void;
}
"#,
        );
        test.result(result).assert_no_lint("no-empty-interface");
    }
}
