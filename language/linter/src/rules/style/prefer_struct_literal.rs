use destack_ast::{self as ast, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer struct literal syntax over constructor calls.
    ///
    /// Structs are value types and should be constructed using the struct
    /// literal syntax for clarity. Using `new` with structs suggests class
    /// instantiation semantics which is misleading.
    ///
    /// ```
    /// // bad
    /// const p = new Point(1, 2)
    ///
    /// // good
    /// const p = Point { x: 1, y: 2 }
    /// ```
    ///
    /// Note: This lint only flags `new` expressions with simple type paths
    /// (uppercase first letter convention for struct types).
    #[lint(
        id = "prefer-struct-literal",
        code = "LY068",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferStructLiteral,
    "Prefer struct literal syntax"
}

impl LintRule for PreferStructLiteral {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferStructLiteral::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let Expression::New { left, .. } = expression else {
                continue;
            };

            // check if the callee looks like a struct type (simple path, PascalCase)
            if is_struct_like_type(ctx, *left) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        PREFER_STRUCT_LITERAL.id,
                        PREFER_STRUCT_LITERAL.code,
                        PREFER_STRUCT_LITERAL.category,
                        severity,
                        "prefer struct literal syntax over `new` constructor",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use `Type { field: value }` instead"),
                );
            }
        }
    }
}

/// Check if the expression looks like a struct type (PascalCase name).
fn is_struct_like_type(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);

    let Expression::Path { path, .. } = expression else {
        return false;
    };

    // get the last segment of the path (the type name)
    let Some(last_segment) = path.segments.last() else {
        return false;
    };

    let name = ctx.strings.get(*last_segment);
    let name_str = name.as_ref();

    // check if it starts with uppercase (PascalCase convention for types)
    name_str
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_new_struct_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferStructLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
const p = new Point(1, 2)
"#,
        );
        test.result(result).assert_lint("prefer-struct-literal");
    }

    #[test]
    fn test_struct_literal_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferStructLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
const p = Point { x: 1, y: 2 }
"#,
        );
        test.result(result).assert_no_lint("prefer-struct-literal");
    }

    #[test]
    fn test_new_namespaced_struct_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferStructLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
const p = new geom.Point(1, 2)
"#,
        );
        test.result(result).assert_lint("prefer-struct-literal");
    }

    #[test]
    fn test_new_lowercase_allowed() {
        let test = TestProgram::for_rule_without_builtins(PreferStructLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
const p = new factory(1, 2)
"#,
        );
        // lowercase names are likely functions, not struct constructors
        test.result(result).assert_no_lint("prefer-struct-literal");
    }

    #[test]
    fn test_multiple_new_calls_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferStructLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
const a = new Point(1, 2)
const b = new Vector(3, 4)
"#,
        );
        // both should be detected
        test.result(result)
            .assert_lint_count("prefer-struct-literal", 2);
    }

    #[test]
    fn test_new_without_args_detected() {
        let test = TestProgram::for_rule_without_builtins(PreferStructLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
const p = new Point()
"#,
        );
        test.result(result).assert_lint("prefer-struct-literal");
    }
}
