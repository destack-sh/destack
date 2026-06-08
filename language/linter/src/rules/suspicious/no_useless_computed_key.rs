use crate::LintMeta;
use destack_dir as dir;
use destack_repository::LintSeverity;
use destack_source::Span;

use crate::rules::common::span_has_comment;
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary computed property keys in objects.
    ///
    /// Using a computed key with a static string or number literal is unnecessary
    /// when a direct property key preserves the same behavior.
    #[lint(
        id = "no-useless-computed-key",
        code = "LU036",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessComputedKey,
    "Disallow useless computed keys"
}

impl LintRule for NoUselessComputedKey {
    fn meta(&self) -> &'static LintMeta {
        NoUselessComputedKey::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Property>() {
            let property = ctx.dir.get(node_id);
            let key = match property {
                dir::Property::Field { key, .. } => key,
                dir::Property::Method { key: Some(key), .. } => key,
                _ => continue,
            };

            // check for computed key with string literal
            let dir::Key::Expression(expr_id) = key else {
                continue;
            };

            let expr = ctx.dir.get(*expr_id);
            let Some((label_text, replacement_text)) = computed_key_replacement(ctx, expr) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, *expr_id);
            if !severity.is_enabled() {
                continue;
            }

            let property_span = ctx.dir.get_span(node_id);
            let mut diagnostic = LintReport::new(
                NO_USELESS_COMPUTED_KEY.id,
                NO_USELESS_COMPUTED_KEY.code,
                NO_USELESS_COMPUTED_KEY.category,
                severity,
                "useless computed key",
                property_span,
            )
            .label(format!("use `{label_text}` without a computed key"));

            if ctx.compute_fixes
                && let Some(key_span) = computed_key_bracket_span(ctx, *expr_id)
                && !span_has_comment(ctx.dir.tree(), key_span)
            {
                let edits = ctx
                    .edit_builder()
                    .replace(key_span, replacement_text)
                    .into_edits();
                let fix = LintFix::safe("Convert to static key").with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return label and replacement text for one useless computed key expression.
fn computed_key_replacement(
    ctx: &LintModuleContext<'_>,
    expression: &dir::Expression,
) -> Option<(String, String)> {
    match expression {
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(string_id)) => {
            let string_text = ctx.strings.get(*string_id);
            if string_text == "__proto__" {
                return None;
            }

            let quoted = format!("\"{string_text}\"");
            Some((quoted.clone(), quoted))
        }
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::Integer(value)) => {
            let value_text = value.to_string();
            Some((value_text.clone(), value_text))
        }
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::Float(value)) => {
            let value_text = value.to_string();
            Some((value_text.clone(), value_text))
        }
        _ => None,
    }
}

/// Return one span that covers `[<expression>]` around a computed key expression.
fn computed_key_bracket_span(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<Span> {
    let source = ctx.source_text().as_bytes();
    let expression_span = ctx.dir.get_span(expression_id);

    let mut left_cursor = expression_span.start as usize;
    while left_cursor > 0 && source[left_cursor - 1].is_ascii_whitespace() {
        left_cursor -= 1;
    }
    if left_cursor == 0 || source[left_cursor - 1] != b'[' {
        return None;
    }
    let key_start = left_cursor - 1;

    let mut right_cursor = expression_span.end as usize;
    while right_cursor < source.len() && source[right_cursor].is_ascii_whitespace() {
        right_cursor += 1;
    }
    if right_cursor >= source.len() || source[right_cursor] != b']' {
        return None;
    }
    let key_end = right_cursor + 1;

    Some(Span::new(
        expression_span.file,
        key_start as u32,
        key_end as u32,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_computed_string_key() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_detects_computed_string_key.ds",
            r#"
const obj = { ["x"]: 1 }
"#,
        );
        test.result(result).assert_lint("no-useless-computed-key");
    }

    #[test]
    fn test_detects_computed_string_key_multi_char() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_detects_computed_string_key_multi_char.ds",
            r#"
const obj = { ["foo"]: 1 }
"#,
        );
        test.result(result).assert_lint("no-useless-computed-key");
    }

    #[test]
    fn test_detects_computed_numeric_key() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_detects_computed_numeric_key.ds",
            r#"
const obj = { [0]: 1 }
"#,
        );
        test.result(result)
            .assert_lint("no-useless-computed-key")
            .assert_safe_fixed(
                r#"
const obj = { 0: 1 };
"#,
            );
    }

    #[test]
    fn test_allows_proto_computed_key() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_allows_proto_computed_key.ds",
            r#"
const obj = { ["__proto__"]: value }
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-computed-key");
    }

    #[test]
    fn test_detects_non_identifier_computed_key() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_detects_non_identifier_computed_key.ds",
            r#"
const obj = { ["Content-Type"]: "json" }
"#,
        );
        test.result(result)
            .assert_lint("no-useless-computed-key")
            .assert_safe_fixed(
                r#"
const obj = { "Content-Type": "json" };
"#,
            );
    }

    #[test]
    fn test_allows_variable_computed_key() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_allows_variable_computed_key.ds",
            r#"
const key = "x"
const obj = { [key]: 1 }
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-computed-key");
    }

    #[test]
    fn test_allows_static_key() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_allows_static_key.ds",
            r#"
const obj = { x: 1 }
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-computed-key");
    }

    #[test]
    fn test_detects_numeric_string_key() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_detects_numeric_string_key.ds",
            r#"
const obj = { ["123"]: 1 }
"#,
        );
        test.result(result)
            .assert_lint("no-useless-computed-key")
            .assert_safe_fixed(
                r#"
const obj = { "123": 1 };
"#,
            );
    }

    #[test]
    fn test_fix_computed_to_static() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_fix_computed_to_static.ds",
            r#"
const obj = { ["foo"]: 1 }
"#,
        );
        test.result(result)
            .assert_lint("no-useless-computed-key")
            .assert_safe_fixed(
                r#"
const obj = { "foo": 1 };
"#,
            );
    }

    #[test]
    fn test_has_no_fix_when_computed_key_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(NoUselessComputedKey);
        let result = test.lint(
            "no_useless_computed_key/test_has_no_fix_when_computed_key_contains_comment.ds",
            r#"
const obj = { [/* keep */ "foo"]: 1 }
"#,
        );
        test.result(result)
            .assert_lint("no-useless-computed-key")
            .assert_has_no_fix("no-useless-computed-key");
    }
}
