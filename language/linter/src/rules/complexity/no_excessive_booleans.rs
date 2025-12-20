use destack_ast::{self as ast, Declaration, Member, Parameter, TypeLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow too many boolean parameters or struct fields.
    ///
    /// Functions with many boolean parameters are confusing to call because
    /// you can't tell what each `true` or `false` means at the call site.
    /// Similarly, structs with many boolean fields can be hard to understand.
    ///
    /// Consider using an enum or options object instead.
    ///
    /// bad: `fn process(a: bool, b: bool, c: bool, d: bool) { ... }`
    /// good: `fn process(options: ProcessOptions) { ... }`
    #[lint(
        id = "no-excessive-booleans",
        code = "LX009",
        category = Complexity,
        level = Ast,
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
        let meta = self.meta();
        let max_booleans = ctx.options.max_booleans;

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);

            // check function parameters
            if let Declaration::Function { signature, .. } = declaration {
                let bool_count = signature
                    .dynamic_parameters
                    .iter()
                    .filter(|param_id| {
                        let param = ctx.tree.get(**param_id);
                        is_param_bool_type(ctx, param)
                    })
                    .count();

                if bool_count > max_booleans {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    ctx.report(
                        LintDiagnostic::new(
                            NO_EXCESSIVE_BOOLEANS.id,
                            NO_EXCESSIVE_BOOLEANS.code,
                            NO_EXCESSIVE_BOOLEANS.category,
                            severity,
                            format!(
                                "function has {bool_count} boolean parameters (max {max_booleans})"
                            ),
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("consider using an options object or enum"),
                    );
                }
            }

            // check struct fields
            if let Declaration::Struct { members, .. } = declaration {
                let bool_count = members
                    .iter()
                    .filter(|member_id| {
                        let member = ctx.tree.get(**member_id);
                        is_member_bool_type(ctx, member)
                    })
                    .count();

                if bool_count > max_booleans {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    ctx.report(
                        LintDiagnostic::new(
                            NO_EXCESSIVE_BOOLEANS.id,
                            NO_EXCESSIVE_BOOLEANS.code,
                            NO_EXCESSIVE_BOOLEANS.category,
                            severity,
                            format!("struct has {bool_count} boolean fields (max {max_booleans})"),
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("consider using an enum or bitflags"),
                    );
                }
            }
        }
    }
}

/// Return whether a parameter has boolean type.
fn is_param_bool_type(ctx: &LintModuleAstContext<'_>, param: &Parameter) -> bool {
    let ty = match param {
        Parameter::Named { ty, .. } => *ty,
        Parameter::Pattern { ty, .. } => *ty,
        Parameter::Variadic { ty, .. } => *ty,
    };
    is_bool_type(ctx, ty)
}

/// Return whether a member has boolean type.
fn is_member_bool_type(ctx: &LintModuleAstContext<'_>, member: &Member) -> bool {
    match member {
        Member::Field { value, .. } => is_bool_type(ctx, *value),
        _ => false,
    }
}

/// Return whether a type annotation is a boolean type.
fn is_bool_type(
    ctx: &LintModuleAstContext<'_>,
    annotation: Option<ast::LocalNodeId<ast::Expression>>,
) -> bool {
    let Some(expr_id) = annotation else {
        return false;
    };
    let expr = ctx.tree.get(expr_id);
    matches!(expr, ast::Expression::TypeLiteral(TypeLiteral::Boolean))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_excessive_boolean_params() {
        let test = TestProgram::for_rule_without_builtins(NoExcessiveBooleans);
        let result = test.lint_ast(
            "test.ds",
            r#"
function process(a: boolean, b: boolean, c: boolean, d: boolean) {
    // four bools is too many
}
"#,
        );
        test.result(result).assert_lint("no-excessive-booleans");
    }

    #[test]
    fn test_allows_few_boolean_params() {
        let test = TestProgram::for_rule_without_builtins(NoExcessiveBooleans);
        let result = test.lint_ast(
            "test.ds",
            r#"
function process(a: boolean, b: boolean, c: boolean) {
    // three bools is ok
}
"#,
        );
        test.result(result).assert_no_lint("no-excessive-booleans");
    }

    #[test]
    fn test_detects_excessive_boolean_fields() {
        let test = TestProgram::for_rule_without_builtins(NoExcessiveBooleans);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct Options {
    enabled: boolean,
    visible: boolean,
    active: boolean,
    selected: boolean,
}
"#,
        );
        test.result(result).assert_lint("no-excessive-booleans");
    }

    #[test]
    fn test_allows_few_boolean_fields() {
        let test = TestProgram::for_rule_without_builtins(NoExcessiveBooleans);
        let result = test.lint_ast(
            "test.ds",
            r#"
struct Options {
    enabled: boolean,
    visible: boolean,
    active: boolean,
}
"#,
        );
        test.result(result).assert_no_lint("no-excessive-booleans");
    }

    #[test]
    fn test_allows_non_boolean_params() {
        let test = TestProgram::for_rule_without_builtins(NoExcessiveBooleans);
        let result = test.lint_ast(
            "test.ds",
            r#"
function process(a: int32, b: string, c: float64, d: int32) {
    // many params but not booleans
}
"#,
        );
        test.result(result).assert_no_lint("no-excessive-booleans");
    }
}
