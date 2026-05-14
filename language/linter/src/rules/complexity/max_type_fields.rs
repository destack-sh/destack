use crate::LintMeta;
use destack_dir::{self as dir, Member, TypeExpression, TypeMember};
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of fields in one type declaration.
    ///
    /// Types with many fields are harder to understand and maintain.
    /// Consider grouping related fields into nested types.
    #[lint(
        id = "max-type-fields",
        code = "LX013",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxTypeFields,
    "Limit type fields"
}

impl LintRule for MaxTypeFields {
    fn meta(&self) -> &'static LintMeta {
        MaxTypeFields::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_type_fields = ctx.options.complexity.max_type_fields;

        // check declaration field counts
        for declaration_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let declaration = ctx.dir.get(declaration_id);

            // check struct class interface member fields
            if let Some((type_kind, field_count)) = declaration_field_count(ctx, declaration) {
                if field_count > max_type_fields {
                    report_type_field_overflow(
                        ctx,
                        meta,
                        declaration_id,
                        type_kind,
                        field_count,
                        max_type_fields,
                    );
                }

                continue;
            }

            // check object type alias field counts
            let dir::Declaration::Type(declaration) = declaration else {
                continue;
            };
            let Some(field_count) = object_type_field_count(ctx, declaration.value) else {
                continue;
            };
            if field_count > max_type_fields {
                report_type_field_overflow(
                    ctx,
                    meta,
                    declaration_id,
                    "type",
                    field_count,
                    max_type_fields,
                );
            }
        }
    }
}

/// Return declaration kind and field count for field carrying declarations.
fn declaration_field_count(
    ctx: &LintModuleContext<'_>,
    declaration: &dir::Declaration,
) -> Option<(&'static str, usize)> {
    // resolve declaration members when present
    match declaration {
        dir::Declaration::Struct(declaration) => {
            let field_count = declaration
                .members
                .iter()
                .filter(|member_id| matches!(ctx.dir.get(**member_id), Member::Field { .. }))
                .count();

            Some(("struct", field_count))
        }
        dir::Declaration::Class(declaration) => {
            let field_count = declaration
                .members
                .iter()
                .filter(|member_id| matches!(ctx.dir.get(**member_id), Member::Field { .. }))
                .count();

            Some(("class", field_count))
        }
        dir::Declaration::Interface(declaration) => {
            let field_count = declaration
                .members
                .iter()
                .filter(|member_id| matches!(ctx.dir.get(**member_id), TypeMember::Field { .. }))
                .count();

            Some(("interface", field_count))
        }
        _ => None,
    }
}

/// Return object type field count for one type expression when it is an object type.
fn object_type_field_count(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> Option<usize> {
    let type_expression_id =
        unwrap_parenthesized_type_expression(ctx.dir.tree(), type_expression_id);
    let TypeExpression::Object { members } = ctx.dir.get(type_expression_id) else {
        return None;
    };

    // count field style members only
    let field_count = members
        .iter()
        .filter(|member_id| matches!(ctx.dir.get(**member_id), TypeMember::Field { .. }))
        .count();

    Some(field_count)
}

/// Return the type expression id with parenthesized wrappers removed.
fn unwrap_parenthesized_type_expression(
    tree: &dir::Tree,
    mut type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> dir::LocalNodeId<dir::TypeExpression> {
    loop {
        let TypeExpression::Parenthesized { expression } = tree.get(type_expression_id) else {
            return type_expression_id;
        };

        type_expression_id = *expression;
    }
}

/// Report one field count overflow diagnostic.
fn report_type_field_overflow<T: dir::Node + Clone>(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: dir::LocalNodeId<T>,
    type_kind: &str,
    field_count: usize,
    max_type_fields: usize,
) where
    dir::Tree: dir::TreeStore<T>,
{
    // resolve owner span before moving owner id into severity lookup
    let owner_span = ctx.dir.get_span(owner_id);

    // resolve effective severity
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // emit one field count overflow diagnostic
    ctx.report(
        LintReport::new(
            MAX_TYPE_FIELDS.id,
            MAX_TYPE_FIELDS.code,
            MAX_TYPE_FIELDS.category,
            severity,
            format!("{type_kind} has {field_count} fields (max {max_type_fields})"),
            owner_span,
        )
        .label("consider grouping related fields"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_struct_fields() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeFields)
            .with_options(|options| options.complexity.max_type_fields = 3);
        let result = test.lint(
            "max_type_fields/test_detects_too_many_struct_fields.ds",
            r#"
struct TooMany {
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
    fn test_detects_too_many_class_fields() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeFields)
            .with_options(|options| options.complexity.max_type_fields = 3);
        let result = test.lint(
            "max_type_fields/test_detects_too_many_class_fields.ds",
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
    fn test_ignores_methods_for_field_count() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeFields)
            .with_options(|options| options.complexity.max_type_fields = 2);
        let result = test.lint(
            "max_type_fields/test_ignores_methods_for_field_count.ds",
            r#"
struct WithMethods {
    a: int32;
    b: int32;
    method1(): void {}
    method2(): void {}
}
"#,
        );
        test.result(result).assert_no_lint("max-type-fields");
    }

    #[test]
    fn test_detects_too_many_object_type_fields() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeFields)
            .with_options(|options| options.complexity.max_type_fields = 3);
        let result = test.lint(
            "max_type_fields/test_detects_too_many_object_type_fields.ds",
            r#"
type TooMany = { a: int32, b: int32, c: int32, d: int32 };
"#,
        );
        test.result(result).assert_lint("max-type-fields");
    }

    #[test]
    fn test_ignores_runtime_object_literals() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeFields)
            .with_options(|options| options.complexity.max_type_fields = 1);
        let result = test.lint(
            "max_type_fields/test_ignores_runtime_object_literals.ds",
            r#"
const value = { a: 1, b: 2, c: 3 };
"#,
        );
        test.result(result).assert_no_lint("max-type-fields");
    }
}
