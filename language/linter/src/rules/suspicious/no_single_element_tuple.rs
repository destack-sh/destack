use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Warn on single-element tuples that may be accidental.
    ///
    /// A single-element tuple like `(x,)` is unusual and often indicates a
    /// mistake. If intentional, consider using a newtype instead.
    #[lint(
        id = "no-single-element-tuple",
        code = "LU030",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable
    )]
    pub NoSingleElementTuple,
    "Warn on single-element tuples"
}

impl LintRule for NoSingleElementTuple {
    fn meta(&self) -> &'static LintMeta {
        NoSingleElementTuple::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::TupleExpression { elements } = expression else {
                continue;
            };

            if elements.len() == 1 {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintReport::new(
                    NO_SINGLE_ELEMENT_TUPLE.id,
                    NO_SINGLE_ELEMENT_TUPLE.code,
                    NO_SINGLE_ELEMENT_TUPLE.category,
                    severity,
                    "single-element tuple",
                    ctx.tree.get_span(node_id),
                )
                .label("consider using an array or newtype instead");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = no_single_element_tuple_fix(ctx, node_id, elements[0])
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build one safe fix by replacing a one-element tuple with its element.
fn no_single_element_tuple_fix(
    ctx: &LintAstContext<'_>,
    tuple_id: ast::LocalNodeId<ast::Expression>,
    element_id: ast::LocalNodeId<ast::Argument>,
) -> Option<LintFix> {
    let argument = ctx.tree.get(element_id);
    let element_id = match argument {
        ast::Argument::Positional { value, .. } => *value,
        _ => return None,
    };

    let element_span = ctx.tree.get_span(element_id);
    let element_text = ctx.get_span_text(element_span);
    if element_text.trim().is_empty() {
        return None;
    }

    // preserve grouping semantics by keeping one explicit parenthesized expression
    let replacement = format!("({element_text})");
    let tuple_span = ctx.tree.get_span(tuple_id);
    let edits = ctx
        .edit_builder()
        .replace(tuple_span, replacement)
        .into_edits();
    Some(
        LintFix::suggestion("Replace single-element tuple with grouped expression")
            .with_edits(edits),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_single_element_tuple() {
        let test = TestProgram::for_rule_without_prelude(NoSingleElementTuple);
        let result = test.lint_ast(
            "no_single_element_tuple/test_detects_single_element_tuple.ds",
            "const x = (1,);",
        );
        test.result(result)
            .assert_lint("no-single-element-tuple")
            .assert_has_fix("no-single-element-tuple");
    }

    #[test]
    fn test_allows_multi_element_tuple() {
        let test = TestProgram::for_rule_without_prelude(NoSingleElementTuple);
        let result = test.lint_ast(
            "no_single_element_tuple/test_allows_multi_element_tuple.ds",
            "const x = (1, 2);",
        );
        test.result(result)
            .assert_no_lint("no-single-element-tuple");
    }

    #[test]
    fn test_allows_empty_tuple() {
        let test = TestProgram::for_rule_without_prelude(NoSingleElementTuple);
        let result = test.lint_ast(
            "no_single_element_tuple/test_allows_empty_tuple.ds",
            "const x = ();",
        );
        test.result(result)
            .assert_no_lint("no-single-element-tuple");
    }

    #[test]
    fn test_fix_rewrites_single_element_tuple_literal() {
        let test = TestProgram::for_rule_without_prelude(NoSingleElementTuple);
        let result = test.lint_ast(
            "no_single_element_tuple/test_fix_rewrites_single_element_tuple_literal.ds",
            r#"
const x = (1,)
"#,
        );
        test.result(result)
            .assert_lint("no-single-element-tuple")
            .assert_suggested_fixed(
                r#"
const x = (1);
"#,
            );
    }

    #[test]
    fn test_fix_rewrites_single_element_tuple_object_expression() {
        let test = TestProgram::for_rule_without_prelude(NoSingleElementTuple);
        let result = test.lint_ast(
            "no_single_element_tuple/test_fix_rewrites_single_element_tuple_object_expression.ds",
            r#"
const x = ({ value: 1 },)
"#,
        );
        test.result(result)
            .assert_lint("no-single-element-tuple")
            .assert_suggested_fixed(
                r#"
const x = ({ value: 1 });
"#,
            );
    }

    #[test]
    fn test_mutation_detects_parenthesized_single_element_tuple() {
        let test = TestProgram::for_rule_without_prelude(NoSingleElementTuple);
        let result = test.lint_ast(
            "no_single_element_tuple/test_mutation_detects_parenthesized_single_element_tuple.ds",
            r#"
const value = ((input + 1),)
"#,
        );
        test.result(result)
            .assert_lint("no-single-element-tuple")
            .assert_suggested_fixed(
                r#"
const value = ((input + 1));
"#,
            );
    }

    #[test]
    fn test_spread_single_element_tuple_has_no_fix() {
        let test = TestProgram::for_rule_without_prelude(NoSingleElementTuple);
        let result = test.lint_ast(
            "no_single_element_tuple/test_spread_single_element_tuple_has_no_fix.ds",
            r#"
const value = (...items,)
"#,
        );
        test.result(result)
            .assert_lint("no-single-element-tuple")
            .assert_has_no_fix("no-single-element-tuple");
    }
}
