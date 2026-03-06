use destack_ast::{self as ast, TypeLiteral};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, callable_owner_span, expression_unwrap_parenthesized_syntax,
    for_each_callable_signature, parameter_type_expression_id,
};
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        NoExcessiveBooleans::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_booleans = ctx.options.max_booleans;

        // check callable parameters across declarations and method owners
        for_each_callable_signature(ctx.tree, |owner_id, signature, _body| {
            // count boolean typed dynamic parameters
            let boolean_parameter_count = signature
                .dynamic_parameters
                .iter()
                .filter(|parameter_id| {
                    let parameter = ctx.tree.get(**parameter_id);
                    parameter_has_boolean_type(ctx, parameter)
                })
                .count();
            if boolean_parameter_count <= max_booleans {
                return;
            }

            // resolve owner span and severity
            let owner_span = callable_owner_span(ctx.tree, owner_id);
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
                LintDiagnostic::new(
                    NO_EXCESSIVE_BOOLEANS.id,
                    NO_EXCESSIVE_BOOLEANS.code,
                    NO_EXCESSIVE_BOOLEANS.category,
                    severity,
                    format!(
                        "function has {boolean_parameter_count} boolean parameters (max {max_booleans})"
                    ),
                    ctx.module.file_id,
                    owner_span,
                )
                .with_label("consider using an options object or enum"),
            );
        });

        // check declaration field counts and object type aliases
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            // resolve declaration node
            let declaration = ctx.tree.get(declaration_id);

            // check struct, class, and interface field boolean counts
            if let Some((type_kind, members)) = declaration_members_with_fields(declaration) {
                let boolean_field_count = count_boolean_member_fields(ctx, members);
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
            let ast::Declaration::Type { value, .. } = declaration else {
                continue;
            };
            let Some(boolean_field_count) = count_boolean_object_type_fields(ctx, *value) else {
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

/// Return declaration members for declarations that expose field members.
fn declaration_members_with_fields(
    declaration: &ast::Declaration,
) -> Option<(&'static str, &[ast::LocalNodeId<ast::Member>])> {
    match declaration {
        ast::Declaration::Struct { members, .. } => Some(("struct", members.as_slice())),
        ast::Declaration::Class { members, .. } => Some(("class", members.as_slice())),
        ast::Declaration::Interface { members, .. } => Some(("interface", members.as_slice())),
        _ => None,
    }
}

/// Report one boolean field count overflow diagnostic.
fn report_boolean_field_overflow<T: ast::Node + Clone>(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    owner_id: ast::LocalNodeId<T>,
    type_kind: &str,
    boolean_field_count: usize,
    max_booleans: usize,
) {
    // resolve owner span before moving owner id into severity lookup
    let owner_span = ctx.tree.get_span(owner_id);

    // resolve effective severity
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // emit one field count overflow diagnostic
    ctx.report(
        LintDiagnostic::new(
            NO_EXCESSIVE_BOOLEANS.id,
            NO_EXCESSIVE_BOOLEANS.code,
            NO_EXCESSIVE_BOOLEANS.category,
            severity,
            format!("{type_kind} has {boolean_field_count} boolean fields (max {max_booleans})"),
            ctx.module.file_id,
            owner_span,
        )
        .with_label("consider grouping flags into enums or dedicated subtypes"),
    );
}

/// Count boolean typed fields in one member list.
fn count_boolean_member_fields(
    ctx: &LintModuleAstContext<'_>,
    members: &[ast::LocalNodeId<ast::Member>],
) -> usize {
    // count member fields with explicit boolean types
    members
        .iter()
        .filter(|member_id| {
            let member = ctx.tree.get(**member_id);
            match member {
                ast::Member::Field {
                    value: Some(value_id),
                    ..
                } => expression_is_boolean_type(ctx, *value_id),
                _ => false,
            }
        })
        .count()
}

/// Count boolean typed fields for one object type expression.
fn count_boolean_object_type_fields(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<usize> {
    // normalize parenthesized type expression wrappers
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let ast::Expression::ObjectExpression { properties, .. } = ctx.tree.get(expression_id) else {
        return None;
    };

    // count object fields with boolean value type annotations
    let boolean_field_count = properties
        .iter()
        .filter(|property_id| {
            let property = ctx.tree.get(**property_id);
            match property {
                ast::Property::Field {
                    value: Some(value_id),
                    ..
                } => expression_is_boolean_type(ctx, *value_id),
                _ => false,
            }
        })
        .count();

    Some(boolean_field_count)
}

/// Return true when one parameter has explicit boolean type.
fn parameter_has_boolean_type(ctx: &LintModuleAstContext<'_>, parameter: &ast::Parameter) -> bool {
    // resolve optional parameter type annotation
    let Some(type_expression_id) = parameter_type_expression_id(parameter) else {
        return false;
    };

    // check normalized type expression shape
    expression_is_boolean_type(ctx, type_expression_id)
}

/// Return true when one expression is the boolean type literal.
fn expression_is_boolean_type(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // normalize parenthesized wrappers around type expressions
    let expression_id = expression_unwrap_parenthesized_syntax(ctx.tree, expression_id);
    let expression = ctx.tree.get(expression_id);

    matches!(
        expression,
        ast::Expression::TypeLiteral(TypeLiteral::Boolean)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_excessive_boolean_params() {
        let test = TestProgram::for_rule_without_prelude(NoExcessiveBooleans);
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
            "no_excessive_booleans/test_allows_non_boolean_params.ds",
            r#"
function process(a: int32, b: string, c: float64, d: int32) {}
"#,
        );
        test.result(result).assert_no_lint("no-excessive-booleans");
    }
}
