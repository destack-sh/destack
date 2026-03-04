use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::argument_value_expression_id;
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleAstContext, LintRule, declare_lint};

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
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoSparseArrays,
    "Disallow sparse arrays and tuples"
}

impl LintRule for NoSparseArrays {
    fn meta(&self) -> &'static LintMeta {
        NoSparseArrays::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let elements = match expression {
                ast::Expression::ArrayExpression { elements } => elements,
                ast::Expression::TupleExpression { elements } => elements,
                _ => continue,
            };

            // check for holes (stub expressions in elements)
            for element_id in elements {
                let value_id = argument_value_expression_id(ctx.tree, *element_id);

                let value = ctx.tree.get(value_id);
                if matches!(value, ast::Expression::Stub) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    let span = ctx.tree.get_span(*element_id);
                    let mut diagnostic = LintDiagnostic::new(
                        NO_SPARSE_ARRAYS.id,
                        NO_SPARSE_ARRAYS.code,
                        NO_SPARSE_ARRAYS.category,
                        severity,
                        "sparse array or tuple with hole",
                        ctx.module.file_id,
                        span,
                    )
                    .with_label("use explicit `undefined` instead of a hole");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = no_sparse_arrays_fix(ctx, value_id)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Build a safe fix by replacing a sparse hole with `undefined`.
fn no_sparse_arrays_fix(
    ctx: &LintModuleAstContext<'_>,
    stub_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let stub_span = ctx.tree.get_span(stub_expression_id);
    let insert_position = stub_span.start.min(stub_span.end);

    let edits = ctx
        .edit_builder()
        .insert(insert_position, "undefined")
        .into_edits();
    Some(LintFix::safe("Replace sparse hole with undefined").with_edits(edits))
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
            .assert_has_fix("no-sparse-arrays");
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
            .assert_has_fix("no-sparse-arrays");
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
    fn test_fix_rewrites_sparse_array_middle_hole() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_fix_rewrites_sparse_array_middle_hole.ts",
            "const arr = [1, , 3];",
        );
        test.result(result)
            .assert_lint("no-sparse-arrays")
            .assert_safe_fixed("const arr = [1, undefined, 3];");
    }

    #[test]
    fn test_fix_rewrites_sparse_array_leading_hole() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_fix_rewrites_sparse_array_leading_hole.ts",
            "const arr = [, 1];",
        );
        test.result(result)
            .assert_lint("no-sparse-arrays")
            .assert_safe_fixed("const arr = [undefined, 1];");
    }

    #[test]
    fn test_mutation_fix_rewrites_sparse_array_with_multiple_holes() {
        let test = TestProgram::for_rule_without_prelude(NoSparseArrays);
        let result = test.lint_ast(
            "no_sparse_arrays/test_mutation_fix_rewrites_sparse_array_with_multiple_holes.ts",
            "const arr = [1, , , 4];",
        );
        test.result(result)
            .assert_lint_count("no-sparse-arrays", 2)
            .assert_safe_fixed("const arr = [1, undefined, undefined, 4];");
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
            .assert_safe_fixed("const arr = [...values, undefined, 2];");
    }
}
