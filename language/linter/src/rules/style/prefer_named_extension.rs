use crate::LintMeta;
use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Prefer named extensions for foreign types.
    ///
    /// When extending types from other modules, use named extensions
    /// for better discoverability and to avoid conflicts.
    #[lint(
        id = "prefer-named-extension",
        code = "LY045",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferNamedExtension,
    "Prefer named extensions"
}

impl LintRule for PreferNamedExtension {
    fn meta(&self) -> &'static LintMeta {
        PreferNamedExtension::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);

            let Declaration::Extension(declaration) = declaration else {
                continue;
            };

            // check if this is an anonymous extension (no name)
            let has_name = declaration.name.is_some();

            if has_name {
                continue;
            }

            // check if the target type appears to be from another module
            // (has a path with more than one segment)
            let is_foreign = matches!(
                ctx.tree.get(declaration.target_type),
                ast::TypeExpression::Reference { path, .. } if path.segments.len() > 1
            );

            if is_foreign {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        PREFER_NAMED_EXTENSION.id,
                        PREFER_NAMED_EXTENSION.code,
                        PREFER_NAMED_EXTENSION.category,
                        severity,
                        "prefer named extension for foreign type",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add a name to this extension"),
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
    fn test_anonymous_extension_of_foreign_type_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferNamedExtension);
        let result = test.lint_ast(
            "prefer_named_extension/test_anonymous_extension_of_foreign_type_detected.ds",
            r#"
extension of std.io.File {
    function read() {}
}
"#,
        );
        test.result(result).assert_lint("prefer-named-extension");
    }

    #[test]
    fn test_named_extension_of_foreign_type_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferNamedExtension);
        let result = test.lint_ast(
            "prefer_named_extension/test_named_extension_of_foreign_type_allowed.ds",
            r#"
extension FileHelpers of std.io.File {
    function read() {}
}
"#,
        );
        test.result(result).assert_no_lint("prefer-named-extension");
    }

    #[test]
    fn test_anonymous_extension_of_local_type_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferNamedExtension);
        let result = test.lint_ast(
            "prefer_named_extension/test_anonymous_extension_of_local_type_allowed.ds",
            r#"
struct Point { x: int32, y: int32 }

extension of Point {
    function distance() {}
}
"#,
        );
        test.result(result).assert_no_lint("prefer-named-extension");
    }
}
