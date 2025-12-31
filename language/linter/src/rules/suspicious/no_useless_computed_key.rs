use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary computed property keys in objects.
    ///
    /// Using a computed key with a static string literal is unnecessary when the
    /// string is a valid identifier. For example, `{["x"]: 1}` can be written as
    /// `{x: 1}`.
    #[lint(
        id = "no-useless-computed-key",
        code = "LU051",
        category = Suspicious,
        level = Ast,
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessComputedKey,
    "Disallow useless computed keys"
}

impl LintRule for NoUselessComputedKey {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessComputedKey::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Property>() {
            let property = ctx.tree.get(node_id);
            let key = match property {
                ast::Property::Field { key: Some(key), .. } => key,
                ast::Property::Method { key: Some(key), .. } => key,
                _ => continue,
            };

            // check for computed key with string literal
            let ast::Key::Expression(expr_id) = key else {
                continue;
            };

            let expr = ctx.tree.get(*expr_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(string_id)) = expr else {
                continue;
            };

            // get the string value and check if it's a valid identifier
            let string_value = ctx.strings.get(*string_id);
            let string_str = string_value.as_ref();
            if is_valid_identifier(string_str) {
                let severity = ctx.get_effective_severity(meta, *expr_id);
                if !severity.is_enabled() {
                    continue;
                }

                let property_span = ctx.tree.get_span(node_id);

                // make fix: replace `["foo"]` with `.foo`
                // the expression_span is just the string literal, we need to include the brackets
                let expression_span = ctx.tree.get_span(*expr_id);
                // expand span to include surrounding brackets
                let key_span = Span::new(
                    expression_span.file,
                    expression_span.start - 1,
                    expression_span.end + 1,
                );
                let replacement = string_str.to_string();
                let edits = ctx
                    .edit_builder()
                    .replace(key_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Convert to static key").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_COMPUTED_KEY.id,
                        NO_USELESS_COMPUTED_KEY.code,
                        NO_USELESS_COMPUTED_KEY.category,
                        severity,
                        "useless computed key",
                        ctx.module.file_id,
                        property_span,
                    )
                    .with_label(format!(
                        "use `{string_str}` instead of `[\"{string_str}\"]`"
                    ))
                    .with_fix(fix),
                );
            }
        }
    }
}

/// Check if a string is a valid identifier (can be used as a non-computed key).
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    // first char must be letter, underscore, or $
    if !first.is_alphabetic() && first != '_' && first != '$' {
        return false;
    }

    // rest must be alphanumeric, underscore, or $
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_computed_string_key() {
        let test = TestProgram::for_rule_without_builtins(NoUselessComputedKey);
        let result = test.lint_ast(
            "test.ds",
            r#"
const obj = { ["x"]: 1 }
"#,
        );
        test.result(result).assert_lint("no-useless-computed-key");
    }

    #[test]
    fn test_detects_computed_string_key_multi_char() {
        let test = TestProgram::for_rule_without_builtins(NoUselessComputedKey);
        let result = test.lint_ast(
            "test.ds",
            r#"
const obj = { ["foo"]: 1 }
"#,
        );
        test.result(result).assert_lint("no-useless-computed-key");
    }

    #[test]
    fn test_allows_non_identifier_computed_key() {
        let test = TestProgram::for_rule_without_builtins(NoUselessComputedKey);
        let result = test.lint_ast(
            "test.ds",
            r#"
const obj = { ["Content-Type"]: "json" }
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-computed-key");
    }

    #[test]
    fn test_allows_variable_computed_key() {
        let test = TestProgram::for_rule_without_builtins(NoUselessComputedKey);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(NoUselessComputedKey);
        let result = test.lint_ast(
            "test.ds",
            r#"
const obj = { x: 1 }
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-computed-key");
    }

    #[test]
    fn test_allows_numeric_string_key() {
        // numeric strings aren't valid identifiers
        let test = TestProgram::for_rule_without_builtins(NoUselessComputedKey);
        let result = test.lint_ast(
            "test.ds",
            r#"
const obj = { ["123"]: 1 }
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-computed-key");
    }

    #[test]
    fn test_fix_computed_to_static() {
        let test = TestProgram::for_rule_without_builtins(NoUselessComputedKey);
        let result = test.lint_ast(
            "test.ds",
            r#"
const obj = { ["foo"]: 1 }
"#,
        );
        test.result(result)
            .assert_lint("no-useless-computed-key")
            .assert_safe_fixed(
                r#"
const obj = { foo: 1 };
"#,
            );
    }
}
