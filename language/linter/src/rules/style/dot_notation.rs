use crate::LintMeta;
use std::str::FromStr;

use destack_ast::{self as ast, Expression, ScalarLiteral, is_identifier};
use destack_source::Span;
use destack_workspace::LintSeverity;
use regex::Regex;

use crate::rules::common::{expression_static_string_literal_source_form, span_has_comment};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Prefer dot notation over bracket notation for property access.
    ///
    /// Use `obj.property` instead of `obj["property"]` when the property name
    /// is a valid identifier.
    #[lint(
        id = "dot-notation",
        code = "LY009",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub DotNotation,
    "Prefer dot notation for property access"
}

impl LintRule for DotNotation {
    fn meta(&self) -> &'static LintMeta {
        DotNotation::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let allow_pattern =
            ctx.options
                .style
                .dot_notation_allow_pattern
                .as_deref()
                .map(|pattern| {
                    Regex::new(pattern)
                        .expect("validated config invariant: dot-notation allow pattern compiles")
                });

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for index expressions with string literal index
            let Expression::Index {
                left,
                index: Some(index_id),
                ..
            } = expr
            else {
                continue;
            };

            let Some(string_id) = expression_static_string_literal_source_form(ctx.tree, *index_id)
            else {
                continue;
            };

            let property_name = ctx.strings.get(string_id);
            let name_str = property_name.as_ref();

            if !property_name_prefers_dot_notation(ctx, &allow_pattern, name_str) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.tree.get_span(node_id);
            let left_span = ctx.tree.get_span(*left);
            let bracket_span = Span::new(expression_span.file, left_span.end, expression_span.end);
            let mut diagnostic = LintDiagnostic::new(
                DOT_NOTATION.id,
                DOT_NOTATION.code,
                DOT_NOTATION.category,
                severity,
                format!("use `.{name_str}` instead of `[\"{name_str}\"]`"),
                ctx.module.file_id,
                expression_span,
            )
            .with_label("prefer dot notation");

            if ctx.compute_fixes && !span_has_comment(ctx.tree, bracket_span) {
                let left_expression = ctx.tree.get(*left);
                let dot_prefix = if is_numeric_literal_expression(left_expression) {
                    " ."
                } else {
                    "."
                };
                let replacement = format!("{dot_prefix}{name_str}");
                let edits = ctx
                    .edit_builder()
                    .replace(bracket_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Convert to dot notation").with_edits(edits);
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when one property name should prefer dot notation.
fn property_name_prefers_dot_notation(
    ctx: &LintAstContext<'_>,
    allow_pattern: &Option<Regex>,
    name: &str,
) -> bool {
    if !is_identifier(name) {
        return false;
    }

    if ctx.options.style.dot_notation_allow_keywords && ast::Keyword::from_str(name).is_ok() {
        return false;
    }

    if let Some(allow_pattern) = allow_pattern
        && allow_pattern.is_match(name)
    {
        return false;
    }

    true
}

/// Return true when the expression is one numeric literal.
fn is_numeric_literal_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) | ScalarLiteral::Bigint(_)
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_bracket_notation() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_detects_bracket_notation.ds",
            r#"
const x = obj["foo"]
"#,
        );
        test.result(result).assert_lint("dot-notation");
    }

    #[test]
    fn test_allows_dot_notation() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_allows_dot_notation.ds",
            r#"
const x = obj.foo
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_allows_non_identifier_bracket() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_allows_non_identifier_bracket.ds",
            r#"
const x = obj["foo-bar"]
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_allows_numeric_index() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_allows_numeric_index.ds",
            r#"
const x = arr[0]
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_allows_variable_index() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_allows_variable_index.ds",
            r#"
const x = obj[key]
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_detects_underscore_property() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_detects_underscore_property.ds",
            r#"
const x = obj["_private"]
"#,
        );
        test.result(result).assert_lint("dot-notation");
    }

    #[test]
    fn test_fix_bracket_to_dot() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_fix_bracket_to_dot.ds",
            r#"
const x = obj["foo"]
"#,
        );
        test.result(result)
            .assert_lint("dot-notation")
            .assert_safe_fixed(
                r#"
const x = obj.foo;
"#,
            );
    }

    #[test]
    fn test_fix_chained_bracket() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_fix_chained_bracket.ds",
            r#"
const x = obj["foo"]["bar"]
"#,
        );
        // should fix both (reports 2 lints)
        test.result(result)
            .assert_lint_count("dot-notation", 2)
            .assert_safe_fixed(
                r#"
const x = obj.foo.bar;
"#,
            );
    }

    #[test]
    fn test_fix_numeric_literal_receiver() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_fix_numeric_literal_receiver.ds",
            r#"
const x = 1["toString"]
"#,
        );
        test.result(result)
            .assert_lint("dot-notation")
            .assert_safe_fixed(
                r#"
const x = 1 .toString;
"#,
            );
    }

    #[test]
    fn test_allows_keyword_property_when_keywords_are_allowed() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_allows_keyword_property_when_keywords_are_allowed.ds",
            r#"
const x = obj["class"]
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_reports_keyword_property_when_keywords_are_disallowed() {
        let test = TestProgram::for_rule_without_prelude(DotNotation).with_options(|options| {
            options.style.dot_notation_allow_keywords = false;
        });
        let result = test.lint_ast(
            "dot_notation/test_reports_keyword_property_when_keywords_are_disallowed.ds",
            r#"
const x = obj["class"]
"#,
        );
        test.result(result).assert_lint("dot-notation");
    }

    #[test]
    fn test_allows_property_name_matching_allow_pattern() {
        let test = TestProgram::for_rule_without_prelude(DotNotation).with_options(|options| {
            options.style.dot_notation_allow_pattern = Some("^_".to_string());
        });
        let result = test.lint_ast(
            "dot_notation/test_allows_property_name_matching_allow_pattern.ds",
            r#"
const x = obj["_private"]
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_reports_static_template_key() {
        let test = TestProgram::for_rule_without_prelude(DotNotation);
        let result = test.lint_ast(
            "dot_notation/test_reports_static_template_key.ds",
            r#"
const x = obj[`foo`]
"#,
        );
        test.result(result)
            .assert_lint("dot-notation")
            .assert_safe_fixed(
                r#"
const x = obj.foo;
"#,
            );
    }
}
