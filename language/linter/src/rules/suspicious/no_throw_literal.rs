use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_unwrap_parenthesized;
use crate::{LintFix, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow throwing literals.
    ///
    /// Throwing literals loses stack information and is harder to handle.
    #[lint(
        id = "no-throw-literal",
        code = "LU032",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoThrowLiteral,
    "Disallow throwing literal values"
}

impl LintRule for NoThrowLiteral {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
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
            let is_undefined_identifier = thrown_expression_is_undefined_identifier(ctx, thrown_id);
            if !is_literal_expression(thrown) && !is_undefined_identifier {
                continue;
            }

            // honor per node severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // report the diagnostic
            let span = ctx.get_span(node_id);
            let message = if is_undefined_identifier {
                "do not throw undefined"
            } else {
                "throwing a literal value"
            };
            let mut diagnostic = LintReport::new(
                NO_THROW_LITERAL.id,
                NO_THROW_LITERAL.code,
                NO_THROW_LITERAL.category,
                severity,
                message,
                span,
            )
            .label("throw an Error object instead");

            // compute fixes only when requested by the runner
            if ctx.include_fixes
                && let Some(fix) = no_throw_literal_fix(ctx, node_id, thrown_id)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
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

/// Return true when the thrown expression is the bare identifier `undefined`.
fn thrown_expression_is_undefined_identifier(
    ctx: &LintModuleDirContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let undefined_name = ctx.string_id("undefined");
    let expression = ctx.tree.get(expression_id);

    match expression {
        dir::Expression::TypeLiteral {
            value: dir::TypeLiteral::Undefined,
        } => true,
        dir::Expression::UnresolvedPath {
            path,
            generic_arguments,
            ..
        }
        | dir::Expression::LocalReference {
            path,
            generic_arguments,
            ..
        }
        | dir::Expression::ModuleReference {
            path,
            generic_arguments,
            ..
        }
        | dir::Expression::GlobalReference {
            path,
            generic_arguments,
            ..
        } => {
            generic_arguments.is_empty()
                && path.segments.len() == 1
                && path.segments[0] == undefined_name
        }
        _ => false,
    }
}

/// Build one unsafe fix by wrapping a thrown literal in Error.
fn no_throw_literal_fix(
    ctx: &LintModuleDirContext<'_>,
    throw_id: dir::LocalNodeId<dir::Expression>,
    thrown_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let thrown_expression = ctx.tree.get(thrown_id);
    let thrown_span = ctx.get_span(thrown_id);
    let thrown_text = ctx.get_span_text(thrown_span);
    if thrown_text.trim().is_empty() {
        return None;
    }

    let replacement_value = if matches!(
        thrown_expression,
        dir::Expression::ScalarLiteral {
            value: dir::ScalarLiteral::String(_),
            ..
        } | dir::Expression::TemplateExpression { .. }
    ) {
        format!("new Error({thrown_text})")
    } else {
        format!("new Error(String({thrown_text}))")
    };

    let throw_span = ctx.get_span(throw_id);
    let replacement = format!("throw {replacement_value}");
    let edits = ctx
        .edit_builder()
        .replace(throw_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Wrap thrown literal in Error").with_edits(edits))
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
            "no_throw_literal/test_flags_string_literal_throw.ds",
            r#"
throw "oops";
"#,
        );
        test.result(result)
            .assert_lint("no-throw-literal")
            .assert_has_fix("no-throw-literal");
    }

    /// Report throwing object literals.
    #[test]
    fn test_flags_object_literal_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "no_throw_literal/test_flags_object_literal_throw.ds",
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
            "no_throw_literal/test_allows_error_throw.ds",
            r#"
throw new Error("oops");
"#,
        );
        test.result(result).assert_no_lint("no-throw-literal");
    }

    /// Unsafely rewrite string literal throws to Error construction.
    #[test]
    fn test_fix_string_literal_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "no_throw_literal/test_fix_string_literal_throw.ds",
            r#"
throw "oops";
"#,
        );
        test.result(result)
            .assert_lint("no-throw-literal")
            .assert_unsafe_fixed(
                r#"
throw new Error("oops");
"#,
            );
    }

    /// Unsafely rewrite non-string literal throws with string coercion.
    #[test]
    fn test_fix_numeric_literal_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "no_throw_literal/test_fix_numeric_literal_throw.ds",
            r#"
throw 42;
"#,
        );
        test.result(result)
            .assert_lint("no-throw-literal")
            .assert_unsafe_fixed(
                r#"
throw new Error(String(42));
"#,
            );
    }

    /// Mutation: detect and rewrite template literal throws.
    #[test]
    fn test_mutation_fix_template_literal_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "no_throw_literal/test_mutation_fix_template_literal_throw.ds",
            r#"
throw `failed: ${code}`;
"#,
        );
        test.result(result)
            .assert_lint("no-throw-literal")
            .assert_unsafe_fixed(
                r#"
throw new Error(`failed: ${code}`);
"#,
            );
    }

    /// Report and rewrite `throw undefined`.
    #[test]
    fn test_flags_undefined_identifier_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "no_throw_literal/test_flags_undefined_identifier_throw.ds",
            r#"
throw undefined;
"#,
        );
        test.result(result)
            .assert_lint("no-throw-literal")
            .assert_unsafe_fixed(
                r#"
throw new Error(String(undefined));
"#,
            );
    }

    /// Keep qualified names out of bare undefined detection.
    #[test]
    fn test_allows_qualified_undefined_like_throw() {
        let test = TestProgram::for_rule_without_prelude(NoThrowLiteral);
        let result = test.lint_dir(
            "no_throw_literal/test_allows_qualified_undefined_like_throw.ds",
            r#"
throw foo.undefined;
"#,
        );
        test.result(result).assert_no_lint("no-throw-literal");
    }
}
