use destack_ast::{self as ast, Argument, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_static_string_literal_syntax;
use crate::{LintAstContext, LintDiagnostic, LintFix, LintMeta, LintRule, declare_lint};

// TODO #Architecture: this rule works but would be better with canonical DIR symbols

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
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoBlankTarget,
    "Disallow target=\"_blank\" without rel=\"noopener\""
}

impl LintRule for NoBlankTarget {
    fn meta(&self) -> &'static LintMeta {
        NoBlankTarget::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
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

            // check if the tag is supported for target checks
            if !is_checked_target_element(ctx, *left) {
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
            let has_rel_argument = args.iter().any(|arg_id| {
                let arg = ctx.tree.get(*arg_id);
                is_rel_argument(ctx, arg)
            });
            if !has_safe_rel {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let mut diagnostic = LintDiagnostic::new(
                    NO_BLANK_TARGET.id,
                    NO_BLANK_TARGET.code,
                    NO_BLANK_TARGET.category,
                    severity,
                    "target=\"_blank\" without rel=\"noopener\" is a security risk",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("add rel=\"noopener\" or rel=\"noreferrer\"");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && !has_rel_argument
                    && let Some(fix) = blank_target_fix(ctx, args)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Check if the left expression is a checked element tag.
fn is_checked_target_element(
    ctx: &LintAstContext<'_>,
    left: Option<ast::LocalNodeId<Expression>>,
) -> bool {
    let Some(left_id) = left else {
        return false;
    };

    // resolve left expr
    let left_expr = ctx.tree.get(left_id);

    // check for simple path like `a`
    let Expression::Path { path, .. } = left_expr else {
        return false;
    };

    // check if the path has exactly one segment and a checked tag name
    if path.segments.len() == 1 {
        let name_str = ctx.strings.get(path.segments[0]);
        return matches!(name_str.as_ref(), "a" | "area" | "form");
    }

    false
}

/// Check if an argument is `target="_blank"`.
fn is_blank_target(ctx: &LintAstContext<'_>, arg: &Argument) -> bool {
    let Argument::Named { name, value, .. } = arg else {
        return false;
    };

    // check if the name is "target"
    let name_str = ctx.strings.get(name.string());
    if name_str.as_ref() != "target" {
        return false;
    }

    // check if the value is "_blank"
    let Some(string_id) = argument_static_string_id(ctx, *value) else {
        return false;
    };

    // resolve value str
    let value_str = ctx.strings.get(string_id);
    value_str.as_ref() == "_blank"
}

/// Check if an argument is `rel` containing "noopener" or "noreferrer".
fn is_safe_rel(ctx: &LintAstContext<'_>, arg: &Argument) -> bool {
    let Argument::Named { name, value, .. } = arg else {
        return false;
    };

    // check if the name is "rel"
    let name_str = ctx.strings.get(name.string());
    if name_str.as_ref() != "rel" {
        return false;
    }

    // check if one rel token is safe
    let Some(string_id) = argument_static_string_id(ctx, *value) else {
        return false;
    };

    // resolve rel value
    let rel_value = ctx.strings.get(string_id);
    rel_value
        .as_ref()
        .split_ascii_whitespace()
        .any(|token| token == "noopener" || token == "noreferrer")
}

/// Check if an argument is any `rel=...` attribute.
fn is_rel_argument(ctx: &LintAstContext<'_>, arg: &Argument) -> bool {
    let Argument::Named { name, .. } = arg else {
        return false;
    };

    // resolve name str
    let name_str = ctx.strings.get(name.string());
    name_str.as_ref() == "rel"
}

/// Return one static string argument value.
fn argument_static_string_id(
    ctx: &LintAstContext<'_>,
    value: ast::LocalNodeId<Expression>,
) -> Option<ast::StringId> {
    expression_static_string_literal_syntax(ctx.tree, value)
}

/// Build a safe fix that injects rel noopener and noreferrer.
fn blank_target_fix(
    ctx: &LintAstContext<'_>,
    args: &[ast::LocalNodeId<Argument>],
) -> Option<LintFix> {
    let last_argument = args.last()?;
    let last_span = ctx.tree.get_span(*last_argument);
    let edits = ctx
        .edit_builder()
        .insert(last_span.end, " rel=\"noopener noreferrer\"")
        .into_edits();
    Some(LintFix::safe("Add rel=\"noopener noreferrer\"").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_blank_target_without_rel() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_detects_blank_target_without_rel.ds",
            r#"
let link = <a href="https://example.com" target="_blank">Click</a>
"#,
        );
        test.result(result).assert_lint("no-blank-target");
    }

    #[test]
    fn test_fix_adds_rel_to_blank_target_without_rel() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_fix_adds_rel_to_blank_target_without_rel.ds",
            r#"
let link = <a href="https://example.com" target="_blank">Click</a>
"#,
        );
        test.result(result)
            .assert_lint("no-blank-target")
            .assert_safe_fixed(
                r#"
let link = <a href="https://example.com" target="_blank" rel="noopener noreferrer">Click</a>;
"#,
            );
    }

    #[test]
    fn test_allows_blank_target_with_noopener() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_blank_target_with_noopener.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="noopener">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_blank_target_with_noreferrer() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_blank_target_with_noreferrer.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="noreferrer">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_blank_target_with_both() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_blank_target_with_both.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="noopener noreferrer">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_no_fix_for_unsafe_rel_value() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_no_fix_for_unsafe_rel_value.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="nofollow">Click</a>
"#,
        );
        test.result(result)
            .assert_lint("no-blank-target")
            .assert_has_no_fix("no-blank-target");
    }

    #[test]
    fn test_allows_no_target() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_no_target.ds",
            r#"
let link = <a href="https://example.com">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_other_target() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_other_target.ds",
            r#"
let link = <a href="https://example.com" target="_self">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_non_anchor_element() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_non_anchor_element.ds",
            r#"
let elem = <div target="_blank">Content</div>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_detects_area_target_without_rel() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_detects_area_target_without_rel.ds",
            r#"
let area = <area href="https://example.com" target="_blank" />
"#,
        );
        test.result(result).assert_lint("no-blank-target");
    }

    #[test]
    fn test_detects_form_target_without_rel() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_detects_form_target_without_rel.ds",
            r#"
let form = <form action="https://example.com" target="_blank"></form>
"#,
        );
        test.result(result).assert_lint("no-blank-target");
    }

    #[test]
    fn test_flags_rel_without_safe_token_boundary() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_flags_rel_without_safe_token_boundary.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="noopenernoreferrer">Click</a>
"#,
        );
        test.result(result).assert_lint("no-blank-target");
    }
}
