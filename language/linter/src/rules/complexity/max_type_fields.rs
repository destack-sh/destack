use destack_ast::{self as ast, Member, Property};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of fields in a struct, class, interface, or object type.
    ///
    /// Types with many fields can be hard to understand and maintain.
    /// Consider grouping related fields into nested types or breaking the type apart.
    #[lint(
        id = "max-type-fields",
        code = "LX019",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxTypeFields,
    "Limit type fields"
}

impl LintRule for MaxTypeFields {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxTypeFields::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_type_fields = ctx.options.max_type_fields;

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            match expression {
                // check struct, class, interface declarations
                ast::Expression::Declaration(declaration_id) => {
                    let declaration = ctx.tree.get(*declaration_id);
                    let (type_kind, members) = match declaration {
                        ast::Declaration::Struct { members, .. } => ("struct", members),
                        ast::Declaration::Class { members, .. } => ("class", members),
                        ast::Declaration::Interface { members, .. } => ("interface", members),
                        _ => continue,
                    };

                    // count only fields, not methods or other members
                    let field_count = members
                        .iter()
                        .filter(|member_id| {
                            let member = ctx.tree.get(**member_id);
                            matches!(member, Member::Field { .. })
                        })
                        .count();

                    if field_count > max_type_fields {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }
                        ctx.report(
                            LintDiagnostic::new(
                                MAX_TYPE_FIELDS.id,
                                MAX_TYPE_FIELDS.code,
                                MAX_TYPE_FIELDS.category,
                                severity,
                                format!(
                                    "{type_kind} has {field_count} fields (max {max_type_fields})"
                                ),
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("consider grouping related fields"),
                        );
                    }
                }

                // check object type expressions (like `type X = { a: string, b: number }`)
                ast::Expression::ObjectExpression { properties, .. } => {
                    // count only fields, not methods or spread
                    let field_count = properties
                        .iter()
                        .filter(|property_id| {
                            let property = ctx.tree.get(**property_id);
                            matches!(property, Property::Field { .. })
                        })
                        .count();
                    if field_count > max_type_fields {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }
                        ctx.report(
                            LintDiagnostic::new(
                                MAX_TYPE_FIELDS.id,
                                MAX_TYPE_FIELDS.code,
                                MAX_TYPE_FIELDS.category,
                                severity,
                                format!(
                                    "object type has {field_count} fields (max {max_type_fields})"
                                ),
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("consider using a named type or grouping related fields"),
                        );
                    }
                }

                _ => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_struct_fields() {
        let test = TestProgram::for_rule(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct TooMany {
    a: int32,
    b: int32,
    c: int32,
    d: int32,
}
"#,
        );
        test.result(result).assert_lint("max-type-fields");
    }

    #[test]
    fn test_detects_too_many_class_fields() {
        let test = TestProgram::for_rule(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
class TooMany {
    a: int32;
    b: int32;
    c: int32;
    d: int32;
}
"#,
        );
        test.result(result).assert_lint("max-type-fields");
    }

    #[test]
    fn test_detects_too_many_interface_fields() {
        let test = TestProgram::for_rule(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
interface TooMany {
    a: int32;
    b: int32;
    c: int32;
    d: int32;
}
"#,
        );
        test.result(result).assert_lint("max-type-fields");
    }

    #[test]
    fn test_allows_few_fields() {
        let test = TestProgram::for_rule(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 5);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct Point {
    x: int32,
    y: int32,
}
"#,
        );
        test.result(result).assert_no_lint("max-type-fields");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct AtLimit {
    a: int32,
    b: int32,
    c: int32,
}
"#,
        );
        test.result(result).assert_no_lint("max-type-fields");
    }

    #[test]
    fn test_ignores_methods() {
        let test = TestProgram::for_rule(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 2);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct WithMethods {
    a: int32,
    b: int32,

    method1(): void {}
    method2(): void {}
    method3(): void {}
}
"#,
        );
        test.result(result).assert_no_lint("max-type-fields");
    }

    #[test]
    fn test_uses_default_limit() {
        let test = TestProgram::for_rule(MaxTypeFields);
        // default is 30, so this should pass
        let result = test.lint_ast(
            "test.ds",
            r#"
struct ManyFields {
    a: int32, b: int32, c: int32, d: int32, e: int32,
    f: int32, g: int32, h: int32, i: int32, j: int32,
}
"#,
        );
        test.result(result).assert_no_lint("max-type-fields");
    }

    #[test]
    fn test_detects_too_many_object_type_fields() {
        let test = TestProgram::for_rule(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
type TooMany = { a: int32, b: int32, c: int32, d: int32 }
"#,
        );
        test.result(result).assert_lint("max-type-fields");
    }

    #[test]
    fn test_allows_few_object_type_fields() {
        let test = TestProgram::for_rule(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 5);
        let result = test.lint_ast(
            "test.ds",
            r#"
type Point = { x: int32, y: int32 }
"#,
        );
        test.result(result).assert_no_lint("max-type-fields");
    }
}
