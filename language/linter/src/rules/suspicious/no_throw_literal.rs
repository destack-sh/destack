use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_unwrap_parenthesized;
use crate::{LintDiagnostic, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow throwing literals.
    ///
    /// Throwing literals loses stack information and is harder to handle.
    #[lint(
        id = "no-throw-literal",
        code = "LU043",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoThrowLiteral,
    "Disallow throwing literal values"
}

impl LintRule for NoThrowLiteral {
    /// Return lint metadata.
    fn meta(&self) -> &'static crate::LintMeta {
        NoThrowLiteral::meta()
    }

    /// Check module DIR nodes for literal throws.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk expressions for throw statements
        for (node_id, expression) in ctx.tree.iter_nodes_of_type::<dir::Expression>() {
            // skip non throw expressions
            let dir::Expression::Throw { value } = expression else {
                continue;
            };

            // resolve the thrown expression
            let thrown_id = expression_unwrap_parenthesized(ctx.tree, *value);
            let thrown = ctx.tree.get(thrown_id);
            if !is_literal_expression(thrown) {
                continue;
            }

            // honor per node severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // report the diagnostic
            let span = ctx.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_THROW_LITERAL.id,
                    NO_THROW_LITERAL.code,
                    NO_THROW_LITERAL.category,
                    severity,
                    "throwing a literal value",
                    ctx.module.file_id,
                    span,
                )
                .with_label("throw an Error object instead"),
            );
        }
    }
}

/// Return true when the expression is a literal value.
fn is_literal_expression(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::ScalarLiteral { .. }
            | dir::Expression::TemplateExpression { .. }
            | dir::Expression::ArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report throwing string literals.
    #[test]
    fn test_flags_string_literal_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "test.ds",
            r#"
throw "oops";
"#,
        );
        test.result(result).assert_lint("no-throw-literal");
    }

    /// Report throwing object literals.
    #[test]
    fn test_flags_object_literal_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "test.ds",
            r#"
throw { message: "oops" };
"#,
        );
        test.result(result).assert_lint("no-throw-literal");
    }

    /// Allow throwing Error objects.
    #[test]
    fn test_allows_error_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "test.ds",
            r#"
throw new Error("oops");
"#,
        );
        test.result(result).assert_no_lint("no-throw-literal");
    }
}
