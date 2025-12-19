use destack_ast::{self as ast, Key, Name};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer object shorthand syntax.
    ///
    /// Use `{ x }` instead of `{ x: x }` when the property name matches
    /// the variable name.
    #[lint(
        id = "object-shorthand",
        code = "LY017",
        category = Style,
        level = Ast,
        fixable = Always,
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

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
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
                let key_str = ctx.strings.get(*key_name);
                let property_span = ctx.tree.get_span(node_id);

                // make fix: replace `x: x` with just `x`
                let replacement = key_str.as_ref().to_string();
                let edits = ctx
                    .edit_builder()
                    .replace(property_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Use shorthand syntax").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        OBJECT_SHORTHAND.id,
                        OBJECT_SHORTHAND.code,
                        OBJECT_SHORTHAND.category,
                        severity,
                        format!("property `{}` can use shorthand syntax", key_str.as_ref()),
                        ctx.module.file_id,
                        property_span,
                    )
                    .with_label("use shorthand `{ x }` instead of `{ x: x }`")
                    .with_fix(fix),
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
    fn test_detects_redundant_property() {
        let test = TestProgram::for_rule(ObjectShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 1
const obj = { x: x }
"#,
        );
        test.result(result).assert_lint("object-shorthand");
    }

    #[test]
    fn test_allows_shorthand() {
        let test = TestProgram::for_rule(ObjectShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 1
const obj = { x }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    #[test]
    fn test_allows_different_names() {
        let test = TestProgram::for_rule(ObjectShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 1
const obj = { y: x }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    #[test]
    fn test_allows_computed_value() {
        let test = TestProgram::for_rule(ObjectShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 1
const obj = { x: x + 1 }
"#,
        );
        test.result(result).assert_no_lint("object-shorthand");
    }

    #[test]
    fn test_fix_shorthand() {
        let test = TestProgram::for_rule(ObjectShorthand);
        let result = test.lint_ast(
            "test.ds",
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
}
