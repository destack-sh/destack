use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer named extensions for foreign types.
    ///
    /// When extending types from other modules, use named extensions
    /// for better discoverability and to avoid conflicts.
    #[lint(
        id = "prefer-named-extension",
        code = "LD015",
        category = Pedantic,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub PreferNamedExtension,
    "Prefer named extensions"
}

impl LintRule for PreferNamedExtension {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferNamedExtension::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);

            let Declaration::Extension {
                descriptor,
                target_type,
                ..
            } = declaration
            else {
                continue;
            };

            // check if this is an anonymous extension (no name)
            let has_name = descriptor.name.is_some();

            if has_name {
                continue;
            }

            // check if the target type appears to be from another module
            // (has a path with more than one segment)
            let target_expr = ctx.tree.get(*target_type);
            let is_foreign = if let ast::Expression::Path { path, .. } = target_expr {
                path.segments.len() > 1
            } else {
                false
            };

            if is_foreign {
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
        let test = TestProgram::for_rule(PreferNamedExtension);
        let result = test.lint_ast(
            "test.ds",
            r#"
extension for std.io.File {
    function read() {}
}
"#,
        );
        test.result(result).assert_lint("prefer-named-extension");
    }

    #[test]
    fn test_named_extension_of_foreign_type_allowed() {
        let test = TestProgram::for_rule(PreferNamedExtension);
        let result = test.lint_ast(
            "test.ds",
            r#"
extension FileHelpers for std.io.File {
    function read() {}
}
"#,
        );
        test.result(result).assert_no_lint("prefer-named-extension");
    }

    #[test]
    fn test_anonymous_extension_of_local_type_allowed() {
        let test = TestProgram::for_rule(PreferNamedExtension);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct Point { x: int32, y: int32 }

extension for Point {
    function distance() {}
}
"#,
        );
        test.result(result).assert_no_lint("prefer-named-extension");
    }
}
