use destack_ast::{self as ast, Declarator, Expression, Pattern, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer tuple destructuring over index access.
    ///
    /// Use `const (x, y) = tuple` instead of `const x = tuple[0]; const y = tuple[1]`.
    /// Destructuring is more readable and makes the intent clearer.
    #[lint(
        id = "prefer-tuple-destructuring",
        code = "LY031",
        category = Style,
        level = Ast
    )]
    pub PreferTupleDestructuring,
    "Prefer destructuring for tuple element access"
}

impl LintRule for PreferTupleDestructuring {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferTupleDestructuring::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for let bindings with single declarator
            let Expression::Let { declarators, .. } = expr else {
                continue;
            };

            // check each declarator
            for declarator_id in declarators {
                let declarator: &Declarator = ctx.tree.get(*declarator_id);
                let pattern = ctx.tree.get(declarator.pattern);

                // only check simple binding patterns (not already destructuring)
                let Pattern::Binding { .. } = pattern else {
                    continue;
                };

                let Some(value_id) = declarator.value else {
                    continue;
                };

                let value = ctx.tree.get(value_id);

                // check if value is an index access with integer literal
                let Expression::Index {
                    index: Some(index_id),
                    ..
                } = value
                else {
                    continue;
                };

                let index_expr = ctx.tree.get(*index_id);

                // flag integer literal indices (like tuple[0], tuple[1])
                let is_integer_index = matches!(
                    index_expr,
                    Expression::ScalarLiteral(ScalarLiteral::Integer(_))
                );

                if is_integer_index {
                    ctx.report(
                        LintDiagnostic::new(
                            PREFER_TUPLE_DESTRUCTURING.id,
                            PREFER_TUPLE_DESTRUCTURING.code,
                            PREFER_TUPLE_DESTRUCTURING.category,
                            severity,
                            "prefer tuple destructuring over index access",
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("use `const (x, ...) = tuple` instead"),
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
    fn test_detects_index_access() {
        let test = TestProgram::for_rule(PreferTupleDestructuring);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = tuple[0]
"#,
        );
        test.result(result)
            .assert_lint("prefer-tuple-destructuring");
    }

    #[test]
    fn test_allows_destructuring() {
        let test = TestProgram::for_rule(PreferTupleDestructuring);
        let result = test.lint_ast(
            "test.ds",
            r#"
const (x, y) = tuple
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-tuple-destructuring");
    }

    #[test]
    fn test_allows_string_index() {
        let test = TestProgram::for_rule(PreferTupleDestructuring);
        // string index is object property access, not tuple
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = obj["key"]
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-tuple-destructuring");
    }

    #[test]
    fn test_allows_variable_index() {
        let test = TestProgram::for_rule(PreferTupleDestructuring);
        // variable index is dynamic, not tuple
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = arr[i]
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-tuple-destructuring");
    }

    #[test]
    fn test_allows_non_let_index() {
        let test = TestProgram::for_rule(PreferTupleDestructuring);
        // index access not in let binding is fine
        let result = test.lint_ast(
            "test.ds",
            r#"
print(tuple[0])
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-tuple-destructuring");
    }
}
