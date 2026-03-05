use destack_ast::{self as ast, Member, Property};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_unwrap_parenthesized_syntax;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of fields in one type declaration.
    ///
    /// Types with many fields are harder to understand and maintain.
    /// Consider grouping related fields into nested types.
    #[lint(
        id = "max-type-fields",
        code = "LX013",
        category = Complexity,
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        MaxTypeFields::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_type_fields = ctx.options.max_type_fields;

        // check declaration field counts
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);

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
            let ast::Declaration::Type { value, .. } = declaration else {
                continue;
            };
            let Some(field_count) = object_type_field_count(ctx, *value) else {
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
    ctx: &LintModuleAstContext<'_>,
    declaration: &ast::Declaration,
) -> Option<(&'static str, usize)> {
    // resolve declaration members when present
    let (type_kind, members) = match declaration {
        ast::Declaration::Struct { members, .. } => ("struct", members),
        ast::Declaration::Class { members, .. } => ("class", members),
        ast::Declaration::Interface { members, .. } => ("interface", members),
        _ => return None,
    };

    // count field members only
    let field_count = members
        .iter()
        .filter(|member_id| matches!(ctx.tree.get(**member_id), Member::Field { .. }))
        .count();

    Some((type_kind, field_count))
}

/// Return object type field count for one type expression when it is an object type.
fn object_type_field_count(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<usize> {
    // normalize parenthesized wrappers
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let ast::Expression::ObjectExpression { properties, .. } = ctx.tree.get(expression_id) else {
        return None;
    };

    // count field style properties only
    let field_count = properties
        .iter()
        .filter(|property_id| matches!(ctx.tree.get(**property_id), Property::Field { .. }))
        .count();

    Some(field_count)
}

/// Report one field count overflow diagnostic.
fn report_type_field_overflow<T: ast::Node + Clone>(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    owner_id: ast::LocalNodeId<T>,
    type_kind: &str,
    field_count: usize,
    max_type_fields: usize,
) {
    // resolve owner span before moving owner id into severity lookup
    let owner_span = ctx.tree.get_span(owner_id.clone());

    // resolve effective severity
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // emit one field count overflow diagnostic
    ctx.report(
        LintDiagnostic::new(
            MAX_TYPE_FIELDS.id,
            MAX_TYPE_FIELDS.code,
            MAX_TYPE_FIELDS.category,
            severity,
            format!("{type_kind} has {field_count} fields (max {max_type_fields})"),
            ctx.module.file_id,
            owner_span,
        )
        .with_label("consider grouping related fields"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_struct_fields() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeFields)
            .with_options(|options| options.max_type_fields = 3);
        let result = test.lint_ast(
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
            .with_options(|options| options.max_type_fields = 3);
        let result = test.lint_ast(
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
            .with_options(|options| options.max_type_fields = 2);
        let result = test.lint_ast(
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
            .with_options(|options| options.max_type_fields = 3);
        let result = test.lint_ast(
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
            .with_options(|options| options.max_type_fields = 1);
        let result = test.lint_ast(
            "max_type_fields/test_ignores_runtime_object_literals.ds",
            r#"
const value = { a: 1, b: 2, c: 3 };
"#,
        );
        test.result(result).assert_no_lint("max-type-fields");
    }
}
