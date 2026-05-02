use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::argument_value_expression_id;
use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow sparse arrays and tuples.
    ///
    /// Sparse arrays with holes (like `[1, , 3]`) are usually unintentional and can
    /// lead to confusing behavior. If intentional, use explicit `undefined` instead.
    #[lint(
        id = "no-sparse-arrays",
        code = "LC026",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoSparseArrays,
    "Disallow sparse arrays and tuples"
}

impl LintRule for NoSparseArrays {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoSparseArrays::meta()
    }

    /// Check module AST nodes for sparse array and tuple holes.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let elements = match expression {
                ast::Expression::ArrayExpression { elements } => elements,
                ast::Expression::TupleExpression { elements } => elements,
                _ => continue,
            };

            // check for holes (stub expressions in elements)
            for element_id in elements {
                let Some(value_id) = argument_value_expression_id(ctx.tree, *element_id) else {
                    continue;
                };

                // resolve value
                let value = ctx.tree.get(value_id);
                if matches!(value, ast::Expression::Stub) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    let span = ctx.tree.get_span(*element_id);
                    let diagnostic = LintReport::new(
                        NO_SPARSE_ARRAYS.id,
                        NO_SPARSE_ARRAYS.code,
                        NO_SPARSE_ARRAYS.category,
                        severity,
                        "sparse array or tuple with hole",
                        span,
                    )
                    .label("use explicit `undefined` instead of a hole");

                    ctx.report(diagnostic);
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
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_allows_dense_array.ts",
            "const arr = [1, 2, 3];",
        );
        test.result(result).assert_no_lint("no-sparse-arrays");
    }

    #[test]
    fn test_allows_array_with_undefined() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_allows_array_with_undefined.ts",
            "const arr = [1, undefined, 3];",
        );
        test.result(result).assert_no_lint("no-sparse-arrays");
    }

    #[test]
    fn test_allows_empty_array() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_allows_empty_array.ts",
            "const arr = [];",
        );
        test.result(result).assert_no_lint("no-sparse-arrays");
    }

    #[test]
    fn test_detects_sparse_array_middle_hole() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_detects_sparse_array_middle_hole.ts",
            "const arr = [1, , 3];",
        );
        test.result(result)
            .assert_lint("no-sparse-arrays")
            .assert_has_no_fix("no-sparse-arrays");
    }

    #[test]
    fn test_detects_sparse_array_leading_hole() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_detects_sparse_array_leading_hole.ts",
            "const arr = [, 1];",
        );
        test.result(result)
            .assert_lint("no-sparse-arrays")
            .assert_has_no_fix("no-sparse-arrays");
    }

    #[test]
    fn test_allows_trailing_comma() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_allows_trailing_comma.ts",
            "const arr = [1, 2, ];",
        );
        test.result(result).assert_no_lint("no-sparse-arrays");
    }

    #[test]
    fn test_flags_sparse_array_middle_hole_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_flags_sparse_array_middle_hole_without_fix.ts",
            "const arr = [1, , 3];",
        );
        test.result(result)
            .assert_lint("no-sparse-arrays")
            .assert_has_no_fix("no-sparse-arrays");
    }

    #[test]
    fn test_flags_sparse_array_leading_hole_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_flags_sparse_array_leading_hole_without_fix.ts",
            "const arr = [, 1];",
        );
        test.result(result)
            .assert_lint("no-sparse-arrays")
            .assert_has_no_fix("no-sparse-arrays");
    }

    #[test]
    fn test_flags_sparse_array_with_multiple_holes_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_flags_sparse_array_with_multiple_holes_without_fix.ts",
            "const arr = [1, , , 4];",
        );
        test.result(result)
            .assert_lint_count("no-sparse-arrays", 2)
            .assert_has_no_fix("no-sparse-arrays");
    }

    #[test]
    fn test_detects_sparse_single_hole_array() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_detects_sparse_single_hole_array.ts",
            "const arr = [,];",
        );
        test.result(result).assert_lint("no-sparse-arrays");
    }

    #[test]
    fn test_detects_sparse_array_before_trailing_comma() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_detects_sparse_array_before_trailing_comma.ts",
            "const arr = [1,,];",
        );
        test.result(result).assert_lint("no-sparse-arrays");
    }

    #[test]
    fn test_detects_nested_sparse_arrays() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_detects_nested_sparse_arrays.ts",
            "const arr = [[,], [1,,2]];",
        );
        test.result(result).assert_lint_count("no-sparse-arrays", 2);
    }

    #[test]
    fn test_allows_mixed_spread_without_hole() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_allows_mixed_spread_without_hole.ts",
            "const arr = [...values, 1];",
        );
        test.result(result).assert_no_lint("no-sparse-arrays");
    }

    #[test]
    fn test_detects_mixed_spread_with_hole() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_detects_mixed_spread_with_hole.ts",
            "const arr = [...values, , 2];",
        );
        test.result(result)
            .assert_lint("no-sparse-arrays")
            .assert_has_no_fix("no-sparse-arrays");
    }
}
