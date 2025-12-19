use destack_ast::{self as ast, Declaration, DependencyMode, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow anonymous default exports.
    ///
    /// Anonymous default exports make it harder to search for usages and
    /// can lead to inconsistent naming across imports. Named exports are
    /// preferred for better discoverability and refactoring support.
    #[lint(
        id = "no-anonymous-default-export",
        code = "LR003",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoAnonymousDefaultExport,
    "Disallow anonymous default exports"
}

impl LintRule for NoAnonymousDefaultExport {
    fn meta(&self) -> &'static crate::LintMeta {
        NoAnonymousDefaultExport::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // check anonymous default export declarations (e.g., `export default function() {}`)
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            // check if this is a default anonymous declaration
            let declaration = ctx.tree.get(declaration_id);
            let (descriptor, is_anonymous) = match declaration {
                Declaration::Function { descriptor, .. } => (descriptor, descriptor.name.is_none()),
                Declaration::Class { descriptor, .. } => (descriptor, descriptor.name.is_none()),
                _ => continue,
            };
            if descriptor.export != Some(DependencyMode::Default) || !is_anonymous {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, declaration_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(declaration_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_ANONYMOUS_DEFAULT_EXPORT.id,
                    NO_ANONYMOUS_DEFAULT_EXPORT.code,
                    NO_ANONYMOUS_DEFAULT_EXPORT.category,
                    severity,
                    "anonymous default export",
                    ctx.module.file_id,
                    span,
                )
                .with_label("assign a name to this export"),
            );
        }

        // check anonymous default export expressions (e.g., `export default { foo: 1 }`)
        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let Expression::Export { items, .. } = expression else {
                continue;
            };

            // check each item in the export
            for item_id in items {
                // check if this is a default anonymous value export
                let item = ctx.tree.get(*item_id);
                if item.mode != DependencyMode::Default || item.value.is_none() {
                    continue;
                }
                let value = ctx.tree.get(item.value.unwrap());
                if matches!(value, Expression::Path { .. }) {
                    continue;
                }

                let severity = ctx.get_effective_severity(meta, expression_id);
                if !severity.is_enabled() {
                    continue;
                }
                let span = ctx.tree.get_span(expression_id);
                ctx.report(
                    LintDiagnostic::new(
                        NO_ANONYMOUS_DEFAULT_EXPORT.id,
                        NO_ANONYMOUS_DEFAULT_EXPORT.code,
                        NO_ANONYMOUS_DEFAULT_EXPORT.category,
                        severity,
                        "anonymous default export",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("assign a name to this export"),
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
    fn test_detects_anonymous_function() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
export default function() {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_anonymous_class() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
export default class {
    foo() {}
}
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_object_literal() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
export default { foo: 1 }
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_literal() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
export default 42
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_detects_arrow_function() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
export default (x) => x * 2
"#,
        );
        test.result(result)
            .assert_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_named_function() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
export default function myFunction() {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_named_class() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
export default class MyClass {
    foo() {}
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_identifier_export() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
const myValue = 42;
export default myValue;
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }

    #[test]
    fn test_allows_named_exports() {
        let test = TestProgram::for_rule(NoAnonymousDefaultExport);
        let result = test.lint_ast(
            "test.ts",
            r#"
export const foo = 1;
export function bar() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-anonymous-default-export");
    }
}
