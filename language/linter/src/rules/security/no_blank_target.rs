use destack_ast::{self as ast, Argument, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

// #Correctness: no-blank-target works but would be better with canonical DIR symbols?

declare_lint! {
    /// Disallow `target="_blank"` without `rel="noopener noreferrer"`.
    ///
    /// When using `target="_blank"`, the new page can access the original
    /// page via `window.opener`. This can be exploited in phishing attacks.
    /// Adding `rel="noopener"` or `rel="noreferrer"` prevents this.
    #[lint(
        id = "no-blank-target",
        code = "LS001",
        category = Security,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoBlankTarget,
    "Disallow target=\"_blank\" without rel=\"noopener\""
}

impl LintRule for NoBlankTarget {
    fn meta(&self) -> &'static crate::LintMeta {
        NoBlankTarget::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check tree expressions (JSX-like)
            let Expression::TreeExpression {
                left,
                arguments,
                elements: _,
            } = expression
            else {
                continue;
            };

            // check if the tag is an anchor <a>
            if !is_anchor_tag(ctx, *left) {
                continue;
            }

            // get arguments if present
            let Some(args) = arguments else {
                continue;
            };

            // check for target="_blank"
            let has_blank_target = args.iter().any(|arg_id| {
                let arg = ctx.tree.get(*arg_id);
                is_blank_target(ctx, arg)
            });
            if !has_blank_target {
                continue;
            }

            // check for rel="noopener" or rel="noreferrer"
            let has_safe_rel = args.iter().any(|arg_id| {
                let arg = ctx.tree.get(*arg_id);
                is_safe_rel(ctx, arg)
            });
            if !has_safe_rel {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_BLANK_TARGET.id,
                        NO_BLANK_TARGET.code,
                        NO_BLANK_TARGET.category,
                        severity,
                        "target=\"_blank\" without rel=\"noopener\" is a security risk",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add rel=\"noopener\" or rel=\"noreferrer\""),
                );
            }
        }
    }
}

/// Check if the left expression is an anchor tag `<a>`.
fn is_anchor_tag(
    ctx: &LintModuleAstContext<'_>,
    left: Option<ast::LocalNodeId<Expression>>,
) -> bool {
    let Some(left_id) = left else {
        return false;
    };

    let left_expr = ctx.tree.get(left_id);

    // check for simple path like `a`
    let Expression::Path { path, .. } = left_expr else {
        return false;
    };

    // check if the path has exactly one segment and it's "a"
    if path.segments.len() == 1 {
        let name_str = ctx.strings.get(path.segments[0]);
        return name_str.as_ref() == "a";
    }

    false
}

/// Check if an argument is `target="_blank"`.
fn is_blank_target(ctx: &LintModuleAstContext<'_>, arg: &Argument) -> bool {
    let Argument::Named { name, value, .. } = arg else {
        return false;
    };

    // check if the name is "target"
    let name_str = ctx.strings.get(name.string());
    if name_str.as_ref() != "target" {
        return false;
    }

    // check if the value is "_blank"
    let value_expr = ctx.tree.get(*value);
    let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = value_expr else {
        return false;
    };
    let value_str = ctx.strings.get(*string_id);
    value_str.as_ref() == "_blank"
}

/// Check if an argument is `rel` containing "noopener" or "noreferrer".
fn is_safe_rel(ctx: &LintModuleAstContext<'_>, arg: &Argument) -> bool {
    let Argument::Named { name, value, .. } = arg else {
        return false;
    };

    // check if the name is "rel"
    let name_str = ctx.strings.get(name.string());
    if name_str.as_ref() != "rel" {
        return false;
    }

    // check if the value contains "noopener" or "noreferrer"
    let value_expr = ctx.tree.get(*value);
    let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = value_expr else {
        return false;
    };

    let value_str = ctx.strings.get(*string_id);
    let rel_value = value_str.as_ref();
    rel_value.contains("noopener") || rel_value.contains("noreferrer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_blank_target_without_rel() {
        let test = TestProgram::for_rule_without_builtins(NoBlankTarget);
        let result = test.lint_ast(
            "test.ds",
            r#"
let link = <a href="https://example.com" target="_blank">Click</a>
"#,
        );
        test.result(result).assert_lint("no-blank-target");
    }

    #[test]
    fn test_allows_blank_target_with_noopener() {
        let test = TestProgram::for_rule_without_builtins(NoBlankTarget);
        let result = test.lint_ast(
            "test.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="noopener">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_blank_target_with_noreferrer() {
        let test = TestProgram::for_rule_without_builtins(NoBlankTarget);
        let result = test.lint_ast(
            "test.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="noreferrer">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_blank_target_with_both() {
        let test = TestProgram::for_rule_without_builtins(NoBlankTarget);
        let result = test.lint_ast(
            "test.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="noopener noreferrer">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_no_target() {
        let test = TestProgram::for_rule_without_builtins(NoBlankTarget);
        let result = test.lint_ast(
            "test.ds",
            r#"
let link = <a href="https://example.com">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_other_target() {
        let test = TestProgram::for_rule_without_builtins(NoBlankTarget);
        let result = test.lint_ast(
            "test.ds",
            r#"
let link = <a href="https://example.com" target="_self">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_non_anchor_element() {
        let test = TestProgram::for_rule_without_builtins(NoBlankTarget);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <div target="_blank">Content</div>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }
}
