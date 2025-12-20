use destack_ast::{self as ast, Expression, ScalarLiteral, is_identifier};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer dot notation over bracket notation for property access.
    ///
    /// Use `obj.property` instead of `obj["property"]` when the property name
    /// is a valid identifier.
    #[lint(
        id = "dot-notation",
        code = "LY020",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub DotNotation,
    "Prefer dot notation for property access"
}

impl LintRule for DotNotation {
    fn meta(&self) -> &'static crate::LintMeta {
        DotNotation::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

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

            let index_expr = ctx.tree.get(*index_id);

            // check if index is a string literal
            let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = index_expr else {
                continue;
            };

            let property_name = ctx.strings.get(*string_id);
            let name_str = property_name.as_ref();

            // check if string is a valid identifier
            if is_identifier(name_str) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                // make fix: convert `obj["property"]` to `obj.property`
                let expression_span = ctx.tree.get_span(node_id);
                let left_span = ctx.tree.get_span(*left);
                let bracket_span =
                    Span::new(expression_span.file, left_span.end, expression_span.end);
                let replacement = format!(".{name_str}");
                let edits = ctx
                    .edit_builder()
                    .replace(bracket_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Convert to dot notation").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        DOT_NOTATION.id,
                        DOT_NOTATION.code,
                        DOT_NOTATION.category,
                        severity,
                        format!("use `.{name_str}` instead of `[\"{name_str}\"]`"),
                        ctx.module.file_id,
                        expression_span,
                    )
                    .with_label("prefer dot notation")
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
    fn test_detects_bracket_notation() {
        let test = TestProgram::for_rule_without_builtins(DotNotation);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = obj["foo"]
"#,
        );
        test.result(result).assert_lint("dot-notation");
    }

    #[test]
    fn test_allows_dot_notation() {
        let test = TestProgram::for_rule_without_builtins(DotNotation);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = obj.foo
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_allows_non_identifier_bracket() {
        let test = TestProgram::for_rule_without_builtins(DotNotation);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = obj["foo-bar"]
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_allows_numeric_index() {
        let test = TestProgram::for_rule_without_builtins(DotNotation);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = arr[0]
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_allows_variable_index() {
        let test = TestProgram::for_rule_without_builtins(DotNotation);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = obj[key]
"#,
        );
        test.result(result).assert_no_lint("dot-notation");
    }

    #[test]
    fn test_detects_underscore_property() {
        let test = TestProgram::for_rule_without_builtins(DotNotation);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = obj["_private"]
"#,
        );
        test.result(result).assert_lint("dot-notation");
    }

    #[test]
    fn test_fix_bracket_to_dot() {
        let test = TestProgram::for_rule_without_builtins(DotNotation);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(DotNotation);
        let result = test.lint_ast(
            "test.ds",
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
}
