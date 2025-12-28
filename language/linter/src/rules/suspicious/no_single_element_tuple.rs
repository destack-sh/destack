use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Warn on single-element tuples that may be accidental.
    ///
    /// A single-element tuple like `(x,)` is unusual and often indicates a
    /// mistake. If intentional, consider using a newtype instead.
    #[lint(
        id = "no-single-element-tuple",
        code = "LU016",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoSingleElementTuple,
    "Warn on single-element tuples"
}

impl LintRule for NoSingleElementTuple {
    fn meta(&self) -> &'static crate::LintMeta {
        NoSingleElementTuple::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
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

                ctx.report(
                    LintDiagnostic::new(
                        NO_SINGLE_ELEMENT_TUPLE.id,
                        NO_SINGLE_ELEMENT_TUPLE.code,
                        NO_SINGLE_ELEMENT_TUPLE.category,
                        severity,
                        "single-element tuple",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("consider using an array or newtype instead"),
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
    fn test_detects_single_element_tuple() {
        let test = TestProgram::for_rule_without_builtins(NoSingleElementTuple);
        let result = test.lint_ast("test.ds", "const x = (1,);");
        test.result(result).assert_lint("no-single-element-tuple");
    }

    #[test]
    fn test_allows_multi_element_tuple() {
        let test = TestProgram::for_rule_without_builtins(NoSingleElementTuple);
        let result = test.lint_ast("test.ds", "const x = (1, 2);");
        test.result(result)
            .assert_no_lint("no-single-element-tuple");
    }

    #[test]
    fn test_allows_empty_tuple() {
        let test = TestProgram::for_rule_without_builtins(NoSingleElementTuple);
        let result = test.lint_ast("test.ds", "const x = ();");
        test.result(result)
            .assert_no_lint("no-single-element-tuple");
    }
}
