use destack_ast as ast;
use destack_dir::{self as dir};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    declaration_has_embedded_types, declaration_has_extends_heritage,
    local_symbol_has_other_declarations, members_are_all_fields,
};
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer struct for data-only classes.
    ///
    /// Classes that only contain fields without constructor or prototype
    /// behavior read better as structs.
    /// Structs make the value semantics explicit and avoid unnecessary class
    /// machinery.
    #[lint(
        id = "prefer-struct",
        code = "LY055",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferStruct,
    "Prefer struct for data-only classes"
}

impl LintRule for PreferStruct {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferStruct::meta()
    }

    /// Check module DIR declarations for data-only classes.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect class declarations only
        for declaration_id in ctx.tree.iter_node_ids_of_type::<dir::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            let dir::Declaration::Class(_) = declaration else {
                continue;
            };

            // keep declarations that still require class semantics
            if !class_is_struct_candidate(ctx, declaration) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, declaration_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                PREFER_STRUCT.id,
                PREFER_STRUCT.code,
                PREFER_STRUCT.category,
                severity,
                "class with only fields should be a struct",
                ctx.module.file_id,
                ctx.get_span(declaration_id),
            )
            .with_label("use struct instead");

            // attach the rewrite only when the declaration has no merge complexity
            if ctx.include_fixes
                && !local_symbol_has_other_declarations(ctx.symbols, declaration.symbol())
                && let Some(source_declaration_id) =
                    ctx.source_node_id::<ast::Declaration>(declaration_id.into_any())
                && let Some(fix) = prefer_struct_fix(ctx, source_declaration_id)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when one class declaration can use struct semantics instead.
fn class_is_struct_candidate(
    ctx: &LintModuleDirContext<'_>,
    declaration: &dir::Declaration,
) -> bool {
    let dir::Declaration::Class(class_declaration) = declaration else {
        return false;
    };

    // keep ambient and abstract declarations out of this style rule
    if class_declaration.ambient.is_ambient() || class_declaration.is_abstract {
        return false;
    }

    // keep empty classes out of this style rule
    if class_declaration.members.is_empty() {
        return false;
    }

    // keep extends and embed inheritance on classes
    if declaration_has_extends_heritage(declaration) || declaration_has_embedded_types(declaration)
    {
        return false;
    }

    // keep non field members out of this style rule
    members_are_all_fields(ctx.tree, &class_declaration.members)
}

/// Build an unsafe fix by replacing the `class` keyword with `struct`.
fn prefer_struct_fix(
    ctx: &LintModuleDirContext<'_>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
) -> Option<LintFix> {
    let declaration_span = ctx.ast.get_span(declaration_id);
    let declaration_text = ctx.get_span_text(declaration_span);
    let class_offset = declaration_text.find("class")?;

    // rewrite only the declaration keyword and keep the rest of the source intact
    let mut replacement = declaration_text.to_string();
    replacement.replace_range(class_offset..class_offset + 5, "struct");

    let edits = ctx
        .edit_builder()
        .replace(declaration_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Convert class declaration to struct").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_data_only_class() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_detects_data_only_class.ds",
            r#"
class Point {
    x: int32 = 0
    y: int32 = 0
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("prefer-struct");
    }

    #[test]
    fn test_fix_converts_data_only_class_to_struct() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_fix_converts_data_only_class_to_struct.ds",
            r#"
class Point {
    x: int32 = 0
    y: int32 = 0
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("prefer-struct")
            .assert_unsafe_fixed(
                r#"
struct Point {
    x: int32 = 0;
    y: int32 = 0;
}
"#,
            );
    }

    #[test]
    fn test_detects_field_only_class_with_interface_implementation() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_detects_field_only_class_with_interface_implementation.ds",
            r#"
interface Named {
    name: string
}

class User implements Named {
    name: string = ""
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("prefer-struct");
    }

    #[test]
    fn test_class_with_method_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_class_with_method_allowed.ds",
            r#"
class Point {
    x: int32 = 0
    y: int32 = 0

    distance(): float64 {
        return 0.0
    }
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("prefer-struct");
    }

    #[test]
    fn test_class_with_extends_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_class_with_extends_allowed.ds",
            r#"
class Point {
    z: int32 = 0

    base(): int32 {
        return 0
    }
}

class Point3D extends Point {
    w: int32 = 0
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("prefer-struct");
    }

    #[test]
    fn test_abstract_class_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_abstract_class_allowed.ds",
            r#"
abstract class Shape {
    id: int32 = 0
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("prefer-struct");
    }

    #[test]
    fn test_struct_not_affected() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_struct_not_affected.ds",
            r#"
struct Point {
    x: int32 = 0
    y: int32 = 0
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("prefer-struct");
    }

    #[test]
    fn test_empty_class_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_empty_class_allowed.ds",
            r#"
class Empty {}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("prefer-struct");
    }

    #[test]
    fn test_fix_converts_private_field_class() {
        let test = TestProgram::for_rule_without_prelude(PreferStruct);
        let result = test.lint_dir(
            "prefer_struct/test_fix_converts_private_field_class.ds",
            r#"
class Repository {
    private token: string = ""
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("prefer-struct")
            .assert_unsafe_fixed(
                r#"
struct Repository {
    private token: string = "";
}
"#,
            );
    }
}
