use destack_ast::{self as ast, Key, Name};
use destack_workspace::LintSeverity;

use crate::rules::common::span_has_comment_trivia;
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Prefer object shorthand syntax.
    ///
    /// Use `{ x }` instead of `{ x: x }` when the property name matches
    /// the variable name.
    #[lint(
        id = "object-shorthand",
        code = "LY027",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub ObjectShorthand,
    "Prefer object shorthand syntax"
}

impl LintRule for ObjectShorthand {
    fn meta(&self) -> &'static crate::LintMeta {
        ObjectShorthand::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Property>() {
            let property = ctx.tree.get(node_id);

            // look for Property::Field with explicit key and value
            let ast::Property::Field {
                key: Some(Key::Name(Name::Identifier(key_name))),
                value: Some(value_id),
                ..
            } = property
            else {
                continue;
            };

            // check if value is a simple path with same name
            let value_expr = ctx.tree.get(*value_id);
            let ast::Expression::Path { path, .. } = value_expr else {
                continue;
            };

            // single-segment path
            if path.segments.len() != 1 {
                continue;
            }

            // compare segment names
            let value_name = path.segments[0];
            if *key_name == value_name {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let key_str = ctx.strings.get(*key_name);
                let property_span = ctx.tree.get_span(node_id);

                let mut diagnostic = LintDiagnostic::new(
                    OBJECT_SHORTHAND.id,
                    OBJECT_SHORTHAND.code,
                    OBJECT_SHORTHAND.category,
                    severity,
                    format!("property `{}` can use shorthand syntax", key_str.as_ref()),
                    ctx.module.file_id,
                    property_span,
                )
                .with_label("use shorthand `{ x }` instead of `{ x: x }`");

                // keep comment sensitive fields out of autofix paths
                if ctx.compute_fixes && !span_has_comment_trivia(ctx.tree, property_span) {
                    let replacement = key_str.as_ref().to_string();
                    let edits = ctx
                        .edit_builder()
                        .replace(property_span, replacement)
                        .into_edits();
                    let fix = LintFix::safe("Use shorthand syntax").with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_redundant_property() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint_ast(
            "object_shorthand/test_detects_redundant_property.ds",
            r#"
const x = 1
const obj = { x: x }
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }

    #[test]
    fn test_allows_shorthand() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint_ast(
            "object_shorthand/test_allows_shorthand.ds",
            r#"
const x = 1
const obj = { x }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    #[test]
    fn test_allows_different_names() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint_ast(
            "object_shorthand/test_allows_different_names.ds",
            r#"
const x = 1
const obj = { y: x }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    /// Fix redundant property shorthand safely.
    #[test]
    fn test_fix_shorthand() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint_ast(
            "object_shorthand/test_fix_shorthand.ds",
            r#"
const x = 1
const obj = { x: x }
"#,
        );
        test.result(result)
            .assert_lint("object-shorthand")
            .assert_safe_fixed(
                r#"
const x = 1;
const obj = { x };
"#,
            );
    }

    #[test]
    fn test_no_fix_when_property_contains_comment_trivia() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint_ast(
            "object_shorthand/test_no_fix_when_property_contains_comment_trivia.ds",
            r#"
const x = 1
const obj = {
    x /* keep */: x
}
"#,
        );
        test.result(result)
            .assert_lint("object-shorthand")
            .assert_has_no_fix("object-shorthand");
    }
}
    /// Flag redundant property shorthand candidates.
    /// Allow shorthand properties.
    /// Allow unrelated property values.
    /// Avoid fixes through comment trivia.

    /// Respect the quoted-key exemption.
    #[test]
    fn test_allows_quoted_key_when_avoid_quotes_is_enabled() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.object_shorthand_avoid_quotes = true;
        });
        let result = test.lint_ast(
            "object_shorthand/test_allows_quoted_key_when_avoid_quotes_is_enabled.ds",
            r#"
const x = 1
const obj = { "x": x }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    /// Flag longform methods when methods mode applies.
    #[test]
    fn test_flags_longform_method() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand);
        let result = test.lint_ast(
            "object_shorthand/test_flags_longform_method.ds",
            r#"
const obj = {
    foo: function() {
        return 1;
    },
}
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }

    /// Ignore constructors when configured.
    #[test]
    fn test_allows_constructor_method_when_ignored() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.object_shorthand_ignore_constructors = true;
        });
        let result = test.lint_ast(
            "object_shorthand/test_allows_constructor_method_when_ignored.ds",
            r#"
const obj = {
    Foo: function() {
        return 1;
    },
}
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    /// Flag shorthand properties in never mode.
    #[test]
    fn test_flags_shorthand_in_never_mode() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.object_shorthand_mode = ObjectShorthandMode::Never;
        });
        let result = test.lint_ast(
            "object_shorthand/test_flags_shorthand_in_never_mode.ds",
            r#"
const x = 1
const obj = { x }
"#,
        );
        test.result(result)
            .assert_lint("object-shorthand")
            .assert_safe_fixed(
                r#"
const x = 1;
const obj = { x: x };
"#,
            );
    }

    /// Flag mixed shorthand in consistent mode.
    #[test]
    fn test_flags_mixed_object_in_consistent_mode() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.object_shorthand_mode = ObjectShorthandMode::Consistent;
        });
        let result = test.lint_ast(
            "object_shorthand/test_flags_mixed_object_in_consistent_mode.ds",
            r#"
const x = 1
const y = 2
const obj = { x, y: y }
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }

    /// Flag all-longform reducible objects in consistent-as-needed mode.
    #[test]
    fn test_flags_all_longform_object_in_consistent_as_needed_mode() {
        let test = TestProgram::for_rule_without_prelude(ObjectShorthand).with_options(|options| {
            options.object_shorthand_mode = ObjectShorthandMode::ConsistentAsNeeded;
        });
        let result = test.lint_ast(
            "object_shorthand/test_flags_all_longform_object_in_consistent_as_needed_mode.ds",
            r#"
const x = 1
const y = 2
const obj = { x: x, y: y }
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }
