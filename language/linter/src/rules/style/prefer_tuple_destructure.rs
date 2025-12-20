use std::collections::HashMap;

use destack_ast::{self as ast, Declarator, Expression, Pattern, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer tuple destructuring over indexed access.
    ///
    /// When accessing multiple elements of a tuple by index in consecutive
    /// declarations, prefer using tuple destructuring instead.
    ///
    /// ```
    /// // bad
    /// const first = tuple[0]
    /// const second = tuple[1]
    ///
    /// // good
    /// const (first, second) = tuple
    /// ```
    #[lint(
        id = "prefer-tuple-destructure",
        code = "LY060",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferTupleDestructure,
    "Prefer tuple destructuring over indexed access"
}

impl LintRule for PreferTupleDestructure {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferTupleDestructure::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // track indexed accesses by source variable: source_name -> [(index, node_id)]
        let mut indexed_accesses: HashMap<String, Vec<(i64, ast::LocalNodeId<Declarator>)>> =
            HashMap::new();

        // first pass: collect all declarators with indexed access
        for node_id in ctx.tree.iter_nodes::<ast::Declarator>() {
            let declarator = ctx.tree.get(node_id);

            // check if it has a value
            let Some(value_id) = declarator.value else {
                continue;
            };

            let value_expression = ctx.tree.get(value_id);

            // check if it's an indexed access like `tuple[0]`
            let Some((source_name, index)) = get_indexed_access_info(ctx, value_expression) else {
                continue;
            };

            // check if the pattern is a simple binding
            let pattern = ctx.tree.get(declarator.pattern);
            if !matches!(pattern, Pattern::Binding { pattern: None, .. }) {
                continue;
            }

            indexed_accesses
                .entry(source_name)
                .or_default()
                .push((index, node_id));
        }

        // second pass: find sources with multiple indexed accesses
        for (source_name, accesses) in indexed_accesses {
            if accesses.len() >= 2 {
                // report on the first access (suggests converting all of them)
                let (_, first_node_id) = &accesses[0];
                let severity = ctx.get_effective_severity(meta, *first_node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        PREFER_TUPLE_DESTRUCTURE.id,
                        PREFER_TUPLE_DESTRUCTURE.code,
                        PREFER_TUPLE_DESTRUCTURE.category,
                        severity,
                        format!(
                            "multiple indexed accesses to `{source_name}` could use tuple destructuring"
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(*first_node_id),
                    )
                    .with_label("use tuple destructuring instead"),
                );
            }
        }
    }
}

/// Extract the source name and index from an indexed access expression.
fn get_indexed_access_info(
    ctx: &LintModuleAstContext<'_>,
    expression: &Expression,
) -> Option<(String, i64)> {
    let Expression::Index { left, index, .. } = expression else {
        return None;
    };

    // get the source variable name
    let left_expression = ctx.tree.get(*left);
    let Expression::Path { path, .. } = left_expression else {
        return None;
    };

    if path.segments.len() != 1 {
        return None;
    }

    let source_name = ctx.strings.get(path.segments[0]).to_string();

    // get the index value (must be a numeric literal)
    let index_id = (*index)?;
    let index_expression = ctx.tree.get(index_id);
    let Expression::ScalarLiteral(ScalarLiteral::Integer(idx)) = index_expression else {
        return None;
    };

    Some((source_name, *idx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_multiple_indexed_accesses_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferTupleDestructure);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(tuple: (int32, int32)) {
    const first = tuple[0]
    const second = tuple[1]
}
"#,
        );
        test.result(result).assert_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_tuple_destructure_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferTupleDestructure);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(tuple: (int32, int32)) {
    const (first, second) = tuple
}
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_single_access_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferTupleDestructure);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(tuple: (int32, int32)) {
    const first = tuple[0]
}
"#,
        );
        // single access is fine, no need for destructuring
        test.result(result)
            .assert_no_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_different_sources_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferTupleDestructure);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: (int32, int32), b: (int32, int32)) {
    const x = a[0]
    const y = b[0]
}
"#,
        );
        // different source tuples
        test.result(result)
            .assert_no_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_three_accesses_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferTupleDestructure);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(tuple: (int32, int32, int32)) {
    const a = tuple[0]
    const b = tuple[1]
    const c = tuple[2]
}
"#,
        );
        test.result(result).assert_lint("prefer-tuple-destructure");
    }

    #[test]
    fn test_array_access_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferTupleDestructure);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(arr: int32[]) {
    const first = arr[0]
    const second = arr[1]
}
"#,
        );
        // we can't distinguish tuples from arrays at AST level
        // this will trigger the lint (acceptable since arrays can also be destructured)
        test.result(result).assert_lint("prefer-tuple-destructure");
    }
}
