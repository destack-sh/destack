use crate::LintMeta;
use std::collections::HashSet;

use destack_dir::{self as dir, Parameter};
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

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
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub DefaultParamLast,
    "Enforce default parameters to be last"
}

impl LintRule for DefaultParamLast {
    fn meta(&self) -> &'static LintMeta {
        DefaultParamLast::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // iterate over all parameters, deduplicate by parent to check each list once
        let mut seen_parents: HashSet<u32> = HashSet::new();
        for param_id in ctx.dir.iter_nodes::<dir::Parameter>() {
            let Some(parent_id) = ctx.dir.get_parent_id(param_id.id) else {
                continue;
            };

            // skip if we already checked this parent's parameters
            if !seen_parents.insert(parent_id) {
                continue;
            }

            // extract parameter list based on parent node type
            let parent_type = ctx.dir.get_node_type(parent_id);
            let parameters = match parent_type {
                dir::NodeType::Declaration => {
                    let decl_id = dir::LocalNodeId::<dir::Declaration>::new(parent_id);
                    let decl = ctx.dir.get(decl_id);
                    match decl {
                        dir::Declaration::Function(declaration) => {
                            &declaration.signature.parameters
                        }
                        _ => continue,
                    }
                }
                dir::NodeType::Member => {
                    let member_id = dir::LocalNodeId::<dir::Member>::new(parent_id);
                    let member = ctx.dir.get(member_id);
                    match member {
                        dir::Member::Method { signature, .. } => &signature.parameters,
                        _ => continue,
                    }
                }
                dir::NodeType::Property => {
                    let property_id = dir::LocalNodeId::<dir::Property>::new(parent_id);
                    let property = ctx.dir.get(property_id);
                    match property {
                        dir::Property::Method { signature, .. } => &signature.parameters,
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
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    parameters: &[dir::LocalNodeId<Parameter>],
) {
    let mut has_seen_required_parameter = false;

    for parameter_id in parameters.iter().rev() {
        let parameter = ctx.dir.get(*parameter_id);
        let is_required_parameter = parameter_is_required(parameter);

        if is_required_parameter {
            has_seen_required_parameter = true;
            continue;
        }

        if !has_seen_required_parameter {
            continue;
        }

        let severity = ctx.get_effective_severity(meta, *parameter_id);
        if !severity.is_enabled() {
            continue;
        }

        ctx.report(
            LintReport::new(
                DEFAULT_PARAM_LAST.id,
                DEFAULT_PARAM_LAST.code,
                DEFAULT_PARAM_LAST.category,
                severity,
                "default parameter should be last",
                ctx.dir.get_span(*parameter_id),
            )
            .label("move default parameters after required parameters"),
        );
    }
}

/// Return true when one parameter is required.
fn parameter_is_required(parameter: &Parameter) -> bool {
    !parameter_has_default(parameter)
}

/// Return true when one parameter has a default or optional form.
fn parameter_has_default(parameter: &Parameter) -> bool {
    match parameter {
        Parameter::Named {
            is_optional,
            default,
            ..
        }
        | Parameter::Pattern {
            is_optional,
            default,
            ..
        } => default.is_some() || *is_optional,
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => true,
        Parameter::Error => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_non_default_after_default() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint(
            "default_param_last/test_detects_non_default_after_default.ds",
            r#"
function foo(a: int32 = 1, b: int32) {}
"#,
        );
        test.result(result)
            .assert_lint("default-param-last")
            .assert_has_no_fix("default-param-last");
    }

    #[test]
    fn test_detects_non_default_after_default_in_arrow() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint(
            "default_param_last/test_detects_non_default_after_default_in_arrow.ds",
            r#"
let foo = (a: int32 = 1, b: int32) => {}
"#,
        );
        test.result(result)
            .assert_lint("default-param-last")
            .assert_has_no_fix("default-param-last");
    }

    #[test]
    fn test_reports_multiple_defaults_before_required_parameters() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint(
            "default_param_last/test_reports_multiple_defaults_before_required_parameters.ds",
            r#"
function foo(a: int32 = 1, b: int32 = 2, c: int32) {}
"#,
        );
        test.result(result)
            .assert_lint_count("default-param-last", 2)
            .assert_has_no_fix("default-param-last");
    }

    #[test]
    fn test_allows_defaults_last() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "default_param_last/test_allows_variadic_after_default.ds",
            r#"
function foo(a: int32 = 1, ...rest: int32[]) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }

    #[test]
    fn test_reports_optional_parameter_before_required_parameter() {
        let test = TestProgram::for_rule_without_prelude(DefaultParamLast);
        let result = test.lint(
            "default_param_last/test_reports_optional_parameter_before_required_parameter.ds",
            r#"
function foo(a?: int32, b: int32) {}
"#,
        );
        test.result(result).assert_lint("default-param-last");
    }
}
