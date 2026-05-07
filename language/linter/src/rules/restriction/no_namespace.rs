use destack_ast::{self as ast, Declaration, Name, NamespaceForm};
use destack_source::FileType;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow namespace declarations.
    ///
    /// TypeScript namespaces are a legacy feature. Use ES modules (import/export)
    /// instead for better tree shaking and standard module semantics.
    #[lint(
        id = "no-namespace",
        code = "LR017",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoNamespace,
    "Disallow namespace declarations"
}

impl LintRule for NoNamespace {
    fn meta(&self) -> &'static LintMeta {
        NoNamespace::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // declaration file policy
        if namespace_file_is_allowed(ctx.file.ty, ctx) {
            return;
        }

        // inspect candidate declarations
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            if namespace_declaration_is_allowed(ctx, node_id, declaration) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintReport::new(
                    NO_NAMESPACE.id,
                    NO_NAMESPACE.code,
                    NO_NAMESPACE.category,
                    severity,
                    "namespace declaration is not allowed",
                    span,
                )
                .label("use ES modules instead"),
            );
        }
    }
}

/// Return true when namespace declarations should be skipped for one file type.
fn namespace_file_is_allowed(file_type: FileType, ctx: &LintAstContext<'_>) -> bool {
    matches!(
        file_type,
        FileType::DestackDeclaration | FileType::TypeScriptDeclaration
    ) && ctx.options.restriction.allow_namespace_definition_files
}

/// Return true when one namespace declaration is allowed by policy.
fn namespace_declaration_is_allowed(
    ctx: &LintAstContext<'_>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
    declaration: &Declaration,
) -> bool {
    let Declaration::Namespace(declaration) = declaration else {
        return true;
    };

    if namespace_is_external_module(declaration.form, Some(declaration.name)) {
        return true;
    }

    if !ctx.options.restriction.allow_namespace_declarations {
        return false;
    }

    namespace_is_declaration_context(ctx, declaration_id, declaration.is_ambient)
}

/// Return true when one namespace declaration is an external module declaration.
fn namespace_is_external_module(form: NamespaceForm, name: Option<Name>) -> bool {
    form == NamespaceForm::Module && matches!(name, Some(Name::String(_)))
}

/// Return true when one namespace declaration lives in a declaration context.
fn namespace_is_declaration_context(
    ctx: &LintAstContext<'_>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
    is_ambient: bool,
) -> bool {
    if is_ambient {
        return true;
    }

    let mut current = Some(declaration_id.id);

    while let Some(node_id) = current {
        let parent_id = ctx.parents.get_by_id(node_id);
        let Some(parent_id) = parent_id else {
            return false;
        };
        current = Some(parent_id);

        if ctx.tree.get_node_type(parent_id) != ast::NodeType::Declaration {
            continue;
        }

        let parent_declaration_id = ast::LocalNodeId::<ast::Declaration>::new(parent_id);
        let Declaration::Namespace(parent_declaration) = ctx.tree.get(parent_declaration_id) else {
            continue;
        };
        if parent_declaration.is_ambient {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_namespace() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace);
        let result = test.lint_ast(
            "no_namespace/test_detects_namespace.ts",
            r#"
namespace MyNamespace {
    export const foo = 1;
}
"#,
        );
        test.result(result).assert_lint("no-namespace");
    }

    #[test]
    fn test_allows_module_exports() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace);
        let result = test.lint_ast(
            "no_namespace/test_allows_module_exports.ts",
            r#"
export const foo = 1;
export function bar() {}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }

    #[test]
    fn test_allows_external_module_declaration() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace);
        let result = test.lint_ast(
            "no_namespace/test_allows_external_module_declaration.ts",
            r#"
declare module "foo" {
    export const value: number;
}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }

    #[test]
    fn test_detects_declare_namespace_in_source_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace);
        let result = test.lint_ast(
            "no_namespace/test_detects_declare_namespace_in_source_by_default.ts",
            r#"
declare namespace External {
    const value: number;
}
"#,
        );
        test.result(result).assert_lint("no-namespace");
    }

    #[test]
    fn test_allows_declare_namespace_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace).with_options(|options| {
            options.restriction.allow_namespace_declarations = true;
        });
        let result = test.lint_ast(
            "no_namespace/test_allows_declare_namespace_when_enabled.ts",
            r#"
declare namespace External {
    const value: number;
}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }

    #[test]
    fn test_allows_nested_namespace_in_declare_namespace_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace).with_options(|options| {
            options.restriction.allow_namespace_declarations = true;
        });
        let result = test.lint_ast(
            "no_namespace/test_allows_nested_namespace_in_declare_namespace_when_enabled.ts",
            r#"
declare namespace External {
    namespace Inner {
        const value: number;
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }

    #[test]
    fn test_still_detects_namespace_when_enabled_without_declare_context() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace).with_options(|options| {
            options.restriction.allow_namespace_declarations = true;
        });
        let result = test.lint_ast(
            "no_namespace/test_still_detects_namespace_when_enabled_without_declare_context.ts",
            r#"
namespace MyNamespace {
    export const foo = 1;
}
"#,
        );
        test.result(result).assert_lint("no-namespace");
    }

    #[test]
    fn test_skips_declaration_file_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace);
        let result = test.lint_ast(
            "no_namespace/test_skips_declaration_file_by_default.d.ts",
            r#"
declare namespace External {
    const value: number;
}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }

    #[test]
    fn test_includes_declaration_file_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace).with_options(|options| {
            options.include_declaration_files = true;
            options.restriction.allow_namespace_definition_files = false;
        });
        let result = test.lint_ast(
            "no_namespace/test_includes_declaration_file_when_enabled.d.ts",
            r#"
declare namespace External {
    const value: number;
}
"#,
        );
        test.result(result).assert_lint("no-namespace");
    }

    #[test]
    fn test_allows_declaration_file_when_enabled_by_rule_option() {
        let test = TestProgram::for_rule_without_prelude(NoNamespace).with_options(|options| {
            options.include_declaration_files = true;
            options.restriction.allow_namespace_definition_files = true;
        });
        let result = test.lint_ast(
            "no_namespace/test_allows_declaration_file_when_enabled_by_rule_option.d.ts",
            r#"
declare namespace External {
    const value: number;
}
"#,
        );
        test.result(result).assert_no_lint("no-namespace");
    }
}
