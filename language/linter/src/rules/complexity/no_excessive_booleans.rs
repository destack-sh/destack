use crate::LintMeta;
use destack_dir::{self as dir, TypeExpression, TypeLiteral};
use destack_repository::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, callable_owner_span, for_each_callable_signature, parameter_type_expression_id,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow too many boolean parameters or type fields.
    ///
    /// Functions with many boolean parameters are confusing to call because you cannot tell what each `true` or `false` means at the call site.
    /// Similarly, types with many boolean fields can be hard to understand.
    ///
    /// Consider using an enum or options object instead.
    #[lint(
        id = "no-excessive-booleans",
        code = "LX018",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoExcessiveBooleans,
    "Disallow too many boolean params"
}

impl LintRule for NoExcessiveBooleans {
    fn meta(&self) -> &'static LintMeta {
        NoExcessiveBooleans::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_booleans = ctx.options().complexity.max_booleans;

        // check callable parameters across declarations and method owners
        for_each_callable_signature(ctx.dir.tree(), |owner_id, signature, _body| {
            // count boolean typed dynamic parameters
            let boolean_parameter_count = signature
                .parameters
                .iter()
                .filter(|parameter_id| {
                    let parameter = ctx.dir.get(**parameter_id);
                    parameter_has_boolean_type(ctx, parameter)
                })
                .count();
            if boolean_parameter_count <= max_booleans {
                return;
            }

            // resolve owner span and severity
            let owner_span = callable_owner_span(ctx.dir.tree(), owner_id);
            let severity = match owner_id {
                CallableOwnerId::Declaration(declaration_id) => {
                    ctx.get_effective_severity(meta, declaration_id)
                }
                CallableOwnerId::Member(member_id) => ctx.get_effective_severity(meta, member_id),
                CallableOwnerId::Property(property_id) => {
                    ctx.get_effective_severity(meta, property_id)
                }
            };
            if !severity.is_enabled() {
                return;
            }

            // emit one callable boolean parameter overflow diagnostic
            ctx.report(
                LintReport::new(
                    NO_EXCESSIVE_BOOLEANS.id,
                    NO_EXCESSIVE_BOOLEANS.code,
                    NO_EXCESSIVE_BOOLEANS.category,
                    severity,
                    format!(
                        "function has {boolean_parameter_count} boolean parameters (max {max_booleans})"
                    ),
                    owner_span)
                .label("consider using an options object or enum"),
            );
        });

        // check declaration field counts and object type aliases
        for declaration_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            // resolve declaration node
            let declaration = ctx.dir.get(declaration_id);

            // check struct, class, and interface field boolean counts
            if let Some((type_kind, boolean_field_count)) =
                declaration_boolean_field_count(ctx, declaration)
            {
                if boolean_field_count > max_booleans {
                    report_boolean_field_overflow(
                        ctx,
                        meta,
                        declaration_id,
                        type_kind,
                        boolean_field_count,
                        max_booleans,
                    );
                }

                continue;
            }

            // check type alias object field boolean counts
            let dir::Declaration::Type(declaration) = declaration else {
                continue;
            };
            let Some(boolean_field_count) =
                count_boolean_object_type_fields(ctx, declaration.value)
            else {
                continue;
            };
            if boolean_field_count > max_booleans {
                report_boolean_field_overflow(
                    ctx,
                    meta,
                    declaration_id,
                    "type",
                    boolean_field_count,
                    max_booleans,
                );
            }
        }
    }
}

/// Return declaration kind and boolean field count for field-carrying declarations.
fn declaration_boolean_field_count(
    ctx: &LintModuleContext<'_>,
    declaration: &dir::Declaration,
) -> Option<(&'static str, usize)> {
    match declaration {
        dir::Declaration::Struct(declaration) => Some((
            "struct",
            count_boolean_member_fields(ctx, &declaration.members),
        )),
        dir::Declaration::Class(declaration) => Some((
            "class",
            count_boolean_member_fields(ctx, &declaration.members),
        )),
        dir::Declaration::Interface(declaration) => Some((
            "interface",
            count_boolean_type_member_fields(ctx, &declaration.members),
        )),
        _ => None,
    }
}

/// Report one boolean field count overflow diagnostic.
fn report_boolean_field_overflow<T: dir::Node + Clone>(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: dir::LocalNodeId<T>,
    type_kind: &str,
    boolean_field_count: usize,
    max_booleans: usize,
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
            NO_EXCESSIVE_BOOLEANS.id,
            NO_EXCESSIVE_BOOLEANS.code,
            NO_EXCESSIVE_BOOLEANS.category,
            severity,
            format!("{type_kind} has {boolean_field_count} boolean fields (max {max_booleans})"),
            owner_span,
        )
        .label("consider grouping flags into enums or dedicated subtypes"),
    );
}

/// Count boolean typed fields in one member list.
fn count_boolean_member_fields(
    ctx: &LintModuleContext<'_>,
    members: &[dir::LocalNodeId<dir::Member>],
) -> usize {
    // count member fields with explicit boolean types
    members
        .iter()
        .filter(|member_id| {
            let member = ctx.dir.get(**member_id);
            match member {
                dir::Member::Field {
                    declared_type: Some(value_id),
                    ..
                } => expression_is_boolean_type(ctx, *value_id),
                _ => false,
            }
        })
        .count()
}

/// Count boolean typed fields in one type member list.
fn count_boolean_type_member_fields(
    ctx: &LintModuleContext<'_>,
    members: &[dir::LocalNodeId<dir::TypeMember>],
) -> usize {
    // count type members with explicit boolean field types
    members
        .iter()
        .filter(|member_id| {
            let member = ctx.dir.get(**member_id);
            match member {
                dir::TypeMember::Field {
                    declared_type: Some(declared_type),
                    ..
                } => expression_is_boolean_type(ctx, *declared_type),
                _ => false,
            }
        })
        .count()
}

/// Count boolean typed fields for one object type expression.
fn count_boolean_object_type_fields(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> Option<usize> {
    let type_expression_id =
        unwrap_parenthesized_type_expression(ctx.dir.tree(), type_expression_id);
    let dir::TypeExpression::Object { members } = ctx.dir.get(type_expression_id) else {
        return None;
    };

    // count object fields with boolean value type annotations
    let boolean_field_count = members
        .iter()
        .filter(|member_id| {
            let member = ctx.dir.get(**member_id);
            match member {
                dir::TypeMember::Field {
                    declared_type: Some(value_id),
                    ..
                } => expression_is_boolean_type(ctx, *value_id),
                _ => false,
            }
        })
        .count();

    Some(boolean_field_count)
}

/// Return true when one parameter has explicit boolean type.
fn parameter_has_boolean_type(ctx: &LintModuleContext<'_>, parameter: &dir::Parameter) -> bool {
    // resolve optional parameter type annotation
    let Some(type_expression_id) = parameter_type_expression_id(parameter) else {
        return false;
    };

    // check normalized type expression shape
    expression_is_boolean_type(ctx, type_expression_id)
}

/// Return true when one expression is the boolean type literal.
fn expression_is_boolean_type(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let type_expression_id =
        unwrap_parenthesized_type_expression(ctx.dir.tree(), type_expression_id);
    let expression = ctx.dir.get(type_expression_id);

    matches!(
        expression,
        dir::TypeExpression::Literal {
            value: TypeLiteral::Boolean,
        }
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_excessive_boolean_params() {
        let test = TestProgram::for_rule_without_prelude(NoExcessiveBooleans);
        let result = test.lint(
            "no_excessive_booleans/test_detects_excessive_boolean_params.ds",
            r#"
function process(a: boolean, b: boolean, c: boolean, d: boolean) {}
"#,
        );
        test.result(result).assert_lint("no-excessive-booleans");
    }

    #[test]
    fn test_allows_few_boolean_params() {
        let test = TestProgram::for_rule_without_prelude(NoExcessiveBooleans);
        let result = test.lint(
            "no_excessive_booleans/test_allows_few_boolean_params.ds",
            r#"
function process(a: boolean, b: boolean, c: boolean) {}
"#,
        );
        test.result(result).assert_no_lint("no-excessive-booleans");
    }

    #[test]
    fn test_detects_excessive_boolean_method_params() {
        let test = TestProgram::for_rule_without_prelude(NoExcessiveBooleans);
        let result = test.lint(
            "no_excessive_booleans/test_detects_excessive_boolean_method_params.ds",
            r#"
class Service {
    run(a: boolean, b: boolean, c: boolean, d: boolean) {}
}
"#,
        );
        test.result(result).assert_lint("no-excessive-booleans");
    }

    #[test]
    fn test_detects_excessive_boolean_fields() {
        let test = TestProgram::for_rule_without_prelude(NoExcessiveBooleans);
        let result = test.lint(
            "no_excessive_booleans/test_detects_excessive_boolean_fields.ds",
            r#"
struct Options {
    enabled: boolean;
    visible: boolean;
    active: boolean;
    selected: boolean;
}
"#,
        );
        test.result(result).assert_lint("no-excessive-booleans");
    }

    #[test]
    fn test_detects_excessive_boolean_object_type_fields() {
        let test = TestProgram::for_rule_without_prelude(NoExcessiveBooleans);
        let result = test.lint(
            "no_excessive_booleans/test_detects_excessive_boolean_object_type_fields.ds",
            r#"
type Options = {
    enabled: boolean
    visible: boolean
    active: boolean
    selected: boolean
}
"#,
        );
        test.result(result).assert_lint("no-excessive-booleans");
    }

    #[test]
    fn test_allows_non_boolean_params() {
        let test = TestProgram::for_rule_without_prelude(NoExcessiveBooleans);
        let result = test.lint(
            "no_excessive_booleans/test_allows_non_boolean_params.ds",
            r#"
function process(a: int32, b: string, c: float64, d: int32) {}
"#,
        );
        test.result(result).assert_no_lint("no-excessive-booleans");
    }
}
