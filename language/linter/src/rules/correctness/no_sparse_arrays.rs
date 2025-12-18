use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow sparse arrays and tuples.
    ///
    /// Sparse arrays with holes (like `[1, , 3]`) are usually unintentional and can
    /// lead to confusing behavior. If intentional, use explicit `undefined` instead.
    #[lint(
        id = "no-sparse-arrays",
        code = "LC015",
        category = Correctness,
        level = Ast
    )]
    pub NoSparseArrays,
    "Disallow sparse arrays and tuples"
}

impl LintRule for NoSparseArrays {
    fn meta(&self) -> &'static crate::LintMeta {
        NoSparseArrays::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let elements = match expression {
                ast::Expression::ArrayExpression { elements } => elements,
                ast::Expression::TupleExpression { elements } => elements,
                _ => continue,
            };

            // check for holes (stub expressions in elements)
            for element_id in elements {
                let argument = ctx.tree.get(*element_id);
                let value_id = match argument {
                    ast::Argument::Positional { value } => value,
                    ast::Argument::Spread { value } => value,
                    ast::Argument::Named { value, .. } => value,
                    ast::Argument::Labeled { value, .. } => value,
                };

                let value = ctx.tree.get(*value_id);
                if matches!(value, ast::Expression::Stub) {
                    let span = ctx.tree.get_span(*element_id);
                    ctx.report(
                        LintDiagnostic::new(
                            NO_SPARSE_ARRAYS.id,
                            NO_SPARSE_ARRAYS.code,
                            NO_SPARSE_ARRAYS.category,
                            severity,
                            "sparse array or tuple with hole",
                            ctx.module.file_id,
                            span,
                        )
                        .with_label("use explicit `undefined` instead of a hole"),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_allows_dense_array() {
        let test = TestProgram::for_rule(NoSparseArrays);
        let result = test.lint_ast("test.ts", "const arr = [1, 2, 3];");
        test.result(result).assert_no_lint("no-sparse-arrays");
    }

    #[test]
    fn test_allows_array_with_undefined() {
        let test = TestProgram::for_rule(NoSparseArrays);
        let result = test.lint_ast("test.ts", "const arr = [1, undefined, 3];");
        test.result(result).assert_no_lint("no-sparse-arrays");
    }

    #[test]
    fn test_allows_empty_array() {
        let test = TestProgram::for_rule(NoSparseArrays);
        let result = test.lint_ast("test.ts", "const arr = [];");
        test.result(result).assert_no_lint("no-sparse-arrays");
    }

    #[test]
    fn test_detects_sparse_array_middle_hole() {
        let test = TestProgram::for_rule(NoSparseArrays);
        let result = test.lint_ast("test.ts", "const arr = [1, , 3];");
        test.result(result).assert_lint("no-sparse-arrays");
    }

    #[test]
    fn test_detects_sparse_array_leading_hole() {
        let test = TestProgram::for_rule(NoSparseArrays);
        let result = test.lint_ast("test.ts", "const arr = [, 1];");
        test.result(result).assert_lint("no-sparse-arrays");
    }

    #[test]
    fn test_allows_trailing_comma() {
        let test = TestProgram::for_rule(NoSparseArrays);
        let result = test.lint_ast("test.ts", "const arr = [1, 2, ];");
        test.result(result).assert_no_lint("no-sparse-arrays");
    }
}
