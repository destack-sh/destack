use destack_ast::{self as ast, Member};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer struct for data-only classes.
    ///
    /// Classes that only contain fields without any methods are better
    /// expressed as structs, which are simpler and more explicit about
    /// their purpose as plain data containers.
    ///
    /// ```
    /// // bad
    /// class Point {
    ///     x: int32
    ///     y: int32
    /// }
    ///
    /// // good
    /// struct Point {
    ///     x: int32
    ///     y: int32
    /// }
    /// ```
    #[lint(
        id = "prefer-struct",
        code = "LY044",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferStruct,
    "Prefer struct for data-only classes"
}

impl LintRule for PreferStruct {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferStruct::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);

            let ast::Declaration::Class {
                members, heritage, ..
            } = declaration
            else {
                continue;
            };

            // skip classes with extends or implements (they need class semantics)
            if heritage.implements_types.is_some() || heritage.extends_types.is_some() {
                continue;
            }

            // skip empty classes
            if members.is_empty() {
                continue;
            }

            // check if all members are fields (no methods, embeds, or static blocks)
            let all_fields = members.iter().all(|member_id| {
                let member = ctx.tree.get(*member_id);
                matches!(member, Member::Field { .. })
            });

            if all_fields {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        PREFER_STRUCT.id,
                        PREFER_STRUCT.code,
                        PREFER_STRUCT.category,
                        severity,
                        "class with only fields should be a struct",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use struct instead"),
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
    fn test_data_only_class_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferStruct);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Point {
    x: int32
    y: int32
}
"#,
        );
        test.result(result).assert_lint("prefer-struct");
    }

    #[test]
    fn test_class_with_method_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferStruct);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Point {
    x: int32
    y: int32

    distance(): float64 {
        return Math.sqrt(this.x * this.x + this.y * this.y)
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-struct");
    }

    #[test]
    fn test_class_with_extends_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferStruct);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Point3D extends Point {
    z: int32
}
"#,
        );
        test.result(result).assert_no_lint("prefer-struct");
    }

    #[test]
    fn test_struct_not_affected() {
        let test = TestProgram::for_rule_without_builtins(PreferStruct);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct Point {
    x: int32
    y: int32
}
"#,
        );
        test.result(result).assert_no_lint("prefer-struct");
    }

    #[test]
    fn test_empty_class_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferStruct);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Empty {}
"#,
        );
        test.result(result).assert_no_lint("prefer-struct");
    }
}
