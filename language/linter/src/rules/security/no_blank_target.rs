use destack_ast::{self as ast, Argument, Expression};
use destack_workspace::LintSeverity;
use url::Url;

use crate::rules::common::{
    expression_path_segments, expression_static_string_literal_source_form,
};
use crate::{LintAstContext, LintFix, LintMeta, LintReport, LintRule, declare_lint};

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
                generic_arguments: _,
            } = expression
            else {
                continue;
            };

            // check if the tag is supported for target checks
            let Some(target_attribute_name) = checked_target_attribute_name(ctx, *left) else {
                continue;
            };

            // get arguments if present
            let Some(args) = arguments else {
                continue;
            };

            // resolve the target argument position
            let target_argument_index = args.iter().position(|arg_id| {
                let arg = ctx.tree.get(*arg_id);
                is_blank_target(ctx, arg)
            });
            let Some(target_argument_index) = target_argument_index else {
                continue;
            };

            // honor allowed domains before enforcing rel hardening
            if target_url_matches_allowed_domain(
                ctx,
                args,
                target_attribute_name,
                &ctx.options.security.no_blank_target_allow_domains,
            ) {
                continue;
            }

            // resolve explicit rel handling
            let rel_argument_index = args.iter().position(|arg_id| {
                let arg = ctx.tree.get(*arg_id);
                is_rel_argument(ctx, arg)
            });
            let rel_argument_id = rel_argument_index.map(|index| args[index]);
            let rel_status = rel_argument_id.map(|arg_id| {
                rel_safety_status(
                    ctx,
                    arg_id,
                    ctx.options.security.no_blank_target_allow_no_referrer,
                )
            });
            if rel_status == Some(RelSafetyStatus::Safe) {
                continue;
            }

            // accept cases where later spread props may still set or override rel
            if rel_argument_id.is_none()
                && has_trailing_spread_argument(ctx, args, target_argument_index)
            {
                continue;
            }
            if rel_argument_index.is_some_and(|rel_argument_index| {
                has_trailing_spread_argument(ctx, args, target_argument_index)
                    || has_trailing_spread_argument(ctx, args, rel_argument_index)
            }) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let rel_requirement_message = if ctx.options.security.no_blank_target_allow_no_referrer
            {
                "add rel=\"noopener\" or rel=\"noreferrer\""
            } else {
                "add rel=\"noopener\""
            };
            let mut diagnostic = LintReport::new(
                NO_BLANK_TARGET.id,
                NO_BLANK_TARGET.code,
                NO_BLANK_TARGET.category,
                severity,
                "target=\"_blank\" without rel=\"noopener\" is a security risk",
                ctx.tree.get_span(node_id),
            )
            .label(rel_requirement_message);

            // compute fixes only when requested by the runner
            if ctx.compute_fixes
                && let Some(fix) = blank_target_fix(ctx, args, rel_argument_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// One rel attribute safety state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelSafetyStatus {
    /// The rel value already satisfies the rule.
    Safe,
    /// The rel value is static but missing noopener coverage.
    UnsafeStatic,
    /// The rel value exists but is not a static string literal.
    UnsafeDynamic,
}

/// Return the target attribute name for one checked element tag.
fn checked_target_attribute_name(
    ctx: &LintAstContext<'_>,
    left: Option<ast::LocalNodeId<Expression>>,
) -> Option<&'static str> {
    let left_id = left?;

    // check for simple path like `a`
    let path_segments = expression_path_segments(ctx.tree, left_id)?;

    // check if the path has exactly one segment and a checked tag name
    if path_segments.len() == 1 {
        let name_str = ctx.strings.get(path_segments[0]);
        return match name_str {
            "a" | "area" => Some("href"),
            "form" => Some("action"),
            _ => None,
        };
    }

    None
}

/// Check if an argument is `target="_blank"`.
fn is_blank_target(ctx: &LintAstContext<'_>, arg: &Argument) -> bool {
    let Argument::Named { name, value, .. } = arg else {
        return false;
    };

    // check if the name is "target"
    let name_str = ctx.strings.get(name.string());
    if name_str != "target" {
        return false;
    }

    // check if the value is "_blank"
    let Some(string_id) = argument_static_string_id(ctx, *value) else {
        return false;
    };

    // resolve value str
    let value_str = ctx.strings.get(string_id);
    value_str.eq_ignore_ascii_case("_blank")
}

/// Check if an argument is any `rel=...` attribute.
fn is_rel_argument(ctx: &LintAstContext<'_>, arg: &Argument) -> bool {
    let Argument::Named { name, .. } = arg else {
        return false;
    };

    // resolve name str
    let name_str = ctx.strings.get(name.string());
    name_str == "rel"
}

/// Return one static string argument value.
fn argument_static_string_id(
    ctx: &LintAstContext<'_>,
    value: ast::LocalNodeId<Expression>,
) -> Option<ast::StringId> {
    expression_static_string_literal_source_form(ctx.tree, value)
}

/// Return the rel safety status for one rel argument.
fn rel_safety_status(
    ctx: &LintAstContext<'_>,
    argument_id: ast::LocalNodeId<Argument>,
    allow_no_referrer: bool,
) -> RelSafetyStatus {
    let argument = ctx.tree.get(argument_id);
    let Argument::Named { value, .. } = argument else {
        return RelSafetyStatus::UnsafeDynamic;
    };

    let Some(string_id) = argument_static_string_id(ctx, *value) else {
        return RelSafetyStatus::UnsafeDynamic;
    };

    let rel_value = ctx.strings.get(string_id);
    if rel_tokens_are_safe(rel_value.as_ref(), allow_no_referrer) {
        return RelSafetyStatus::Safe;
    }

    RelSafetyStatus::UnsafeStatic
}

/// Return true when one rel value satisfies the rule.
fn rel_tokens_are_safe(rel_value: &str, allow_no_referrer: bool) -> bool {
    let has_noopener = rel_value
        .split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case("noopener"));
    if has_noopener {
        return true;
    }

    allow_no_referrer
        && rel_value
            .split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("noreferrer"))
}

/// Return true when the target attribute matches one allowed domain entry.
fn target_url_matches_allowed_domain(
    ctx: &LintAstContext<'_>,
    args: &[ast::LocalNodeId<Argument>],
    target_attribute_name: &str,
    allowed_domains: &[String],
) -> bool {
    if allowed_domains.is_empty() {
        return false;
    }

    let target_url = args.iter().find_map(|arg_id| {
        let argument = ctx.tree.get(*arg_id);
        static_named_argument_value(ctx, argument, target_attribute_name)
    });
    let Some(target_url_id) = target_url else {
        return false;
    };
    let target_url = ctx.strings.get(target_url_id);
    let target_url = target_url.as_ref();

    allowed_domains
        .iter()
        .any(|allowed_domain| url_matches_allowed_domain(target_url, allowed_domain))
}

/// Return one static named argument string id when present.
fn static_named_argument_value(
    ctx: &LintAstContext<'_>,
    argument: &Argument,
    name: &str,
) -> Option<ast::StringId> {
    let Argument::Named {
        name: argument_name,
        value,
        ..
    } = argument
    else {
        return None;
    };

    if ctx.strings.get(argument_name.string()) != name {
        return None;
    }

    argument_static_string_id(ctx, *value)
}

/// Return true when one target URL matches one allowed domain entry.
fn url_matches_allowed_domain(target_url: &str, allowed_domain: &str) -> bool {
    let target_url = target_url.trim();
    let allowed_domain = allowed_domain.trim();

    if target_url.eq_ignore_ascii_case(allowed_domain) {
        return true;
    }

    match (Url::parse(target_url), Url::parse(allowed_domain)) {
        (Ok(target), Ok(allowed)) => {
            target.scheme().eq_ignore_ascii_case(allowed.scheme())
                && target
                    .host_str()
                    .zip(allowed.host_str())
                    .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
                && target.port_or_known_default() == allowed.port_or_known_default()
        }
        (Ok(target), Err(_)) => target
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case(allowed_domain)),
        (Err(_), Ok(allowed)) => allowed
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case(target_url)),
        (Err(_), Err(_)) => false,
    }
}

/// Return true when one later prop spread may override earlier attributes.
fn has_trailing_spread_argument(
    ctx: &LintAstContext<'_>,
    args: &[ast::LocalNodeId<Argument>],
    start_index: usize,
) -> bool {
    args.iter()
        .skip(start_index + 1)
        .copied()
        .any(|argument_id| matches!(ctx.tree.get(argument_id), Argument::Spread { .. }))
}

/// Build a safe fix that injects or amends rel with noopener.
fn blank_target_fix(
    ctx: &LintAstContext<'_>,
    args: &[ast::LocalNodeId<Argument>],
    rel_argument_id: Option<ast::LocalNodeId<Argument>>,
) -> Option<LintFix> {
    if let Some(rel_argument_id) = rel_argument_id {
        return blank_target_rel_fix(ctx, rel_argument_id);
    }

    let last_argument = args.last()?;
    let last_span = ctx.tree.get_span(*last_argument);
    let edits = ctx
        .edit_builder()
        .insert(last_span.end, " rel=\"noopener\"")
        .into_edits();
    Some(LintFix::safe("Add rel=\"noopener\"").with_edits(edits))
}

/// Build a safe fix that amends one rel attribute with noopener.
fn blank_target_rel_fix(
    ctx: &LintAstContext<'_>,
    rel_argument_id: ast::LocalNodeId<Argument>,
) -> Option<LintFix> {
    let argument = ctx.tree.get(rel_argument_id);
    let Argument::Named { value, .. } = argument else {
        return None;
    };

    let string_id = argument_static_string_id(ctx, *value)?;
    let rel_value = ctx.strings.get(string_id);
    let amended_rel = if rel_value.trim().is_empty() {
        "noopener".to_string()
    } else {
        format!("noopener {rel_value}")
    };

    let value_span = ctx.tree.get_span(*value);
    let value_text = ctx.get_span_text(value_span);
    let replacement = quoted_rel_literal(value_text, &amended_rel)?;
    let edits = ctx
        .edit_builder()
        .replace(value_span, replacement)
        .into_edits();
    Some(LintFix::safe("Add noopener to rel attribute").with_edits(edits))
}

/// Return one rel string literal using the same quote style as the original.
fn quoted_rel_literal(original: &str, rel_value: &str) -> Option<String> {
    let quote = original.chars().next()?;
    if !matches!(quote, '"' | '\'' | '`') || !original.ends_with(quote) {
        return None;
    }

    Some(format!("{quote}{rel_value}{quote}"))
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
let link = <a href="https://example.com" target="_blank" rel="noopener">Click</a>;
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
            .assert_safe_fixed(
                r#"
let link = <a href="https://example.com" target="_blank" rel="noopener nofollow">Click</a>;
"#,
            );
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

    #[test]
    fn test_allows_blank_target_with_case_insensitive_rel_token() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_blank_target_with_case_insensitive_rel_token.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="NoOpener">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_detects_case_insensitive_blank_target() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_detects_case_insensitive_blank_target.ds",
            r#"
let link = <a href="https://example.com" target="_BLANK">Click</a>
"#,
        );
        test.result(result).assert_lint("no-blank-target");
    }

    #[test]
    fn test_allows_blank_target_with_spread_props_without_explicit_rel() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_blank_target_with_spread_props_without_explicit_rel.ds",
            r#"
let link = <a href="https://example.com" target="_blank" {...props}>Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_flags_blank_target_with_leading_spread_props_without_rel() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_flags_blank_target_with_leading_spread_props_without_rel.ds",
            r#"
let link = <a {...props} href="https://example.com" target="_blank">Click</a>
"#,
        );
        test.result(result).assert_lint("no-blank-target");
    }

    #[test]
    fn test_allows_blank_target_with_trailing_spread_after_rel() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget);
        let result = test.lint_ast(
            "no_blank_target/test_allows_blank_target_with_trailing_spread_after_rel.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="nofollow" {...props}>Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_allows_blank_target_for_allowed_domain() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget).with_options(|options| {
            options.security.no_blank_target_allow_domains =
                vec!["https://example.com".to_string()];
        });
        let result = test.lint_ast(
            "no_blank_target/test_allows_blank_target_for_allowed_domain.ds",
            r#"
let link = <a href="https://example.com/path" target="_blank">Click</a>
"#,
        );
        test.result(result).assert_no_lint("no-blank-target");
    }

    #[test]
    fn test_flags_noreferrer_when_no_referrer_is_disabled() {
        let test = TestProgram::for_rule_without_prelude(NoBlankTarget).with_options(|options| {
            options.security.no_blank_target_allow_no_referrer = false;
        });
        let result = test.lint_ast(
            "no_blank_target/test_flags_noreferrer_when_no_referrer_is_disabled.ds",
            r#"
let link = <a href="https://example.com" target="_blank" rel="noreferrer">Click</a>
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
}
