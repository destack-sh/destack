use std::collections::HashSet;

use destack_ast::{self as ast, Parameter};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce default parameters to be last.
    ///
    /// Parameters with default values should come after parameters without
    /// default values. This makes function calls clearer, as you can't skip
    /// defaulted parameters to set later ones.
    #[lint(
        id = "default-param-last",
        code = "LY008",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub DefaultParamLast,
    "Enforce default parameters to be last"
}

impl LintRule for DefaultParamLast {
    fn meta(&self) -> &'static crate::LintMeta {
        DefaultParamLast::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // iterate over all parameters, deduplicate by parent to check each list once
        let mut seen_parents: HashSet<u32> = HashSet::new();
        for param_id in ctx.tree.iter_nodes::<ast::Parameter>() {
            let Some(parent_id) = ctx.parents.get(param_id) else {
                continue;
            };

            // skip if we already checked this parent's parameters
            if !seen_parents.insert(parent_id) {
                continue;
            }

            // extract parameter list based on parent node type
            let parent_type = ctx.tree.get_node_type(parent_id);
            let parameters = match parent_type {
                ast::NodeType::Declaration => {
                    let decl_id = ast::LocalNodeId::<ast::Declaration>::new(parent_id);
                    let decl = ctx.tree.get(decl_id);
                    match decl {
                        ast::Declaration::Function { signature, .. } => {
                            &signature.dynamic_parameters
                        }
                        _ => continue,
                    }
                }
                ast::NodeType::Member => {
                    let member_id = ast::LocalNodeId::<ast::Member>::new(parent_id);
                    let member = ctx.tree.get(member_id);
                    match member {
                        ast::Member::Method { signature, .. } => &signature.dynamic_parameters,
                        _ => continue,
                    }
                }
                ast::NodeType::Property => {
                    let property_id = ast::LocalNodeId::<ast::Property>::new(parent_id);
                    let property = ctx.tree.get(property_id);
                    match property {
                        ast::Property::Method { signature, .. } => &signature.dynamic_parameters,
                        _ => continue,
                    }
                }
                _ => continue,
            };

            check_parameters(ctx, meta, parameters);
        }
    }
}

fn check_parameters(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    parameters: &[ast::LocalNodeId<Parameter>],
) {
    let mut seen_default = false;
    let mut has_reported_reorder_fix = false;

    for param_id in parameters {
        let param = ctx.tree.get(*param_id);

        let has_default = parameter_has_default(param);

        if has_default {
            seen_default = true;
        } else if seen_default {
            // found a param without default after one with default
            // variadic parameters are allowed after defaults
            if !matches!(
                param,
                Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
            ) {
                let severity = ctx.get_effective_severity(meta, *param_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    DEFAULT_PARAM_LAST.id,
                    DEFAULT_PARAM_LAST.code,
                    DEFAULT_PARAM_LAST.category,
                    severity,
                    "parameter without default follows parameter with default",
                    ctx.module.file_id,
                    ctx.tree.get_span(*param_id),
                )
                .with_label("move this parameter before parameters with defaults");
                if ctx.compute_fixes && !has_reported_reorder_fix {
                    if let Some(fix) = default_param_last_fix(ctx, parameters) {
                        diagnostic = diagnostic.with_fix(fix);
                    }
                    has_reported_reorder_fix = true;
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return true when one parameter declares a default value.
fn parameter_has_default(parameter: &Parameter) -> bool {
    match parameter {
        Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => default.is_some(),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => false,
    }
}

/// Build a conservative parameter reorder fix.
fn default_param_last_fix(
    ctx: &LintModuleAstContext<'_>,
    parameters: &[ast::LocalNodeId<Parameter>],
) -> Option<LintFix> {
    // keep at least two parameters
    let first_parameter_id = *parameters.first()?;
    let last_parameter_id = *parameters.last()?;

    // keep comment-free parameter slices
    let first_span = ctx.tree.get_span(first_parameter_id);
    let last_span = ctx.tree.get_span(last_parameter_id);
    let full_span = destack_source::Span::new(first_span.file, first_span.start, last_span.end);
    let full_text = ctx.get_span_text(full_span);
    if full_text.contains("//") || full_text.contains("/*") {
        return None;
    }

    // reorder: non-default first, default next, variadic last
    let mut non_default_ids = Vec::new();
    let mut default_ids = Vec::new();
    let mut variadic_ids = Vec::new();

    for parameter_id in parameters {
        let parameter = ctx.tree.get(*parameter_id);
        match parameter {
            Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => {
                variadic_ids.push(*parameter_id);
            }
            _ if parameter_has_default(parameter) => default_ids.push(*parameter_id),
            _ => non_default_ids.push(*parameter_id),
        }
    }

    let mut ordered_ids = Vec::new();
    ordered_ids.extend(non_default_ids);
    ordered_ids.extend(default_ids);
    ordered_ids.extend(variadic_ids);

    // skip when already ordered
    if ordered_ids.as_slice() == parameters {
        return None;
    }

    let replacement = ordered_ids
        .iter()
        .map(|parameter_id| {
            ctx.get_span_text(ctx.tree.get_span(*parameter_id))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(", ");

    let edits = ctx
        .edit_builder()
        .replace(full_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Reorder parameters so defaults come last").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_non_default_after_default() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint_ast(
            "default_param_last/test_detects_non_default_after_default.ds",
            r#"
function foo(a: int32 = 1, b: int32) {}
"#,
        );
        test.result(result)
            .assert_lint("default-param-last")
            .assert_unsafe_fixed(
                r#"
function foo(b: int32, a: int32 = 1) {}
"#,
            );
    }

    #[test]
    fn test_detects_non_default_after_default_in_arrow() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint_ast(
            "default_param_last/test_detects_non_default_after_default_in_arrow.ds",
            r#"
let foo = (a: int32 = 1, b: int32) => {}
"#,
        );
        test.result(result)
            .assert_lint("default-param-last")
            .assert_unsafe_fixed(
                r#"
let foo = (b: int32, a: int32 = 1) => {};
"#,
            );
    }

    #[test]
    fn test_allows_defaults_last() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint_ast(
            "default_param_last/test_allows_defaults_last.ds",
            r#"
function foo(a: int32, b: int32 = 1) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }

    #[test]
    fn test_allows_all_defaults() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint_ast(
            "default_param_last/test_allows_all_defaults.ds",
            r#"
function foo(a: int32 = 1, b: int32 = 2) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }

    #[test]
    fn test_allows_no_defaults() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint_ast(
            "default_param_last/test_allows_no_defaults.ds",
            r#"
function foo(a: int32, b: int32) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }

    #[test]
    fn test_allows_variadic_after_default() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint_ast(
            "default_param_last/test_allows_variadic_after_default.ds",
            r#"
function foo(a: int32 = 1, ...rest: int32[]) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }

    #[test]
    fn test_no_fix_when_parameter_list_contains_comments() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint_ast(
            "default_param_last/test_no_fix_when_parameter_list_contains_comments.ds",
            r#"
function foo(
    a: int32 = 1, // keep near a
    b: int32
) {}
"#,
        );
        test.result(result)
            .assert_lint("default-param-last")
            .assert_has_no_fix("default-param-last");
    }
}
