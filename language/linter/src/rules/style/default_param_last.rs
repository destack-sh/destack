use std::collections::HashSet;

use destack_ast::{self as ast, Parameter};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce default parameters to be last.
    ///
    /// Parameters with default values should come after parameters without
    /// default values. This makes function calls clearer, as you can't skip
    /// defaulted parameters to set later ones.
    #[lint(
        id = "default-param-last",
        code = "LY010",
        category = Style,
        level = Ast,
        fixable = No,
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

    for param_id in parameters {
        let param = ctx.tree.get(*param_id);

        let has_default = match param {
            Parameter::Named { default, .. } => default.is_some(),
            Parameter::Pattern { default, .. } => default.is_some(),
            Parameter::Variadic { .. } => false, // variadic params don't have defaults
        };

        if has_default {
            seen_default = true;
        } else if seen_default {
            // found a param without default after one with default
            // variadic parameters are allowed after defaults
            if !matches!(param, Parameter::Variadic { .. }) {
                let severity = ctx.get_effective_severity(meta, *param_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        DEFAULT_PARAM_LAST.id,
                        DEFAULT_PARAM_LAST.code,
                        DEFAULT_PARAM_LAST.category,
                        severity,
                        "parameter without default follows parameter with default",
                        ctx.module.file_id,
                        ctx.tree.get_span(*param_id),
                    )
                    .with_label("move this parameter before parameters with defaults"),
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
    fn test_detects_non_default_after_default() {
        let test = TestProgram::for_rule_without_builtins(DefaultParamLast);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: int32 = 1, b: int32) {}
"#,
        );
        test.result(result).assert_lint("default-param-last");
    }

    #[test]
    fn test_detects_non_default_after_default_in_arrow() {
        let test = TestProgram::for_rule_without_builtins(DefaultParamLast);
        let result = test.lint_ast(
            "test.ds",
            r#"
let foo = (a: int32 = 1, b: int32) => {}
"#,
        );
        test.result(result).assert_lint("default-param-last");
    }

    #[test]
    fn test_allows_defaults_last() {
        let test = TestProgram::for_rule_without_builtins(DefaultParamLast);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: int32, b: int32 = 1) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }

    #[test]
    fn test_allows_all_defaults() {
        let test = TestProgram::for_rule_without_builtins(DefaultParamLast);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: int32 = 1, b: int32 = 2) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }

    #[test]
    fn test_allows_no_defaults() {
        let test = TestProgram::for_rule_without_builtins(DefaultParamLast);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: int32, b: int32) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }

    #[test]
    fn test_allows_variadic_after_default() {
        let test = TestProgram::for_rule_without_builtins(DefaultParamLast);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: int32 = 1, ...rest: int32[]) {}
"#,
        );
        test.result(result).assert_no_lint("default-param-last");
    }
}
