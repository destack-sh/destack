use destack_ast::{
    self as ast, BinaryOperator, Expression, LocalNodeId, NodeTree, NodeVisitor,
    NodeVisitorOptions, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Warn on overly complex types that should be aliased.
    ///
    /// Deeply nested generic types, unions, or intersections can be hard to read.
    /// Consider using a type alias to give a name to complex types.
    #[lint(
        id = "no-complex-type",
        code = "LX019",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoComplexType,
    "Warn on overly complex types"
}

impl LintRule for NoComplexType {
    fn meta(&self) -> &'static crate::LintMeta {
        NoComplexType::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_type_complexity = ctx.options.max_type_complexity;

        // Check let/const declarations
        for decl_id in ctx.tree.iter_nodes::<ast::Declarator>() {
            let decl = ctx.tree.get(decl_id);
            if let Some(ty_id) = decl.ty {
                check_type_complexity(ctx, meta, ty_id, max_type_complexity);
            }
        }

        // Check function parameters
        for param_id in ctx.tree.iter_nodes::<ast::Parameter>() {
            let param = ctx.tree.get(param_id);
            let ty_id = match param {
                ast::Parameter::Named { ty, .. }
                | ast::Parameter::Pattern { ty, .. }
                | ast::Parameter::Variadic { ty, .. } => *ty,
            };
            if let Some(ty_id) = ty_id {
                check_type_complexity(ctx, meta, ty_id, max_type_complexity);
            }
        }
    }
}

/// Check if a type expression exceeds the maximum complexity.
fn check_type_complexity(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    ty_id: LocalNodeId<Expression>,
    max_complexity: usize,
) {
    let mut visitor = ComplexityVisitor {
        options: NodeVisitorOptions::default(),
        max_depth: 0,
        current_depth: 0,
    };

    let ty_expr = ctx.tree.get(ty_id);
    visitor.visit_expression(ctx.tree, ty_id, ty_expr);

    if visitor.max_depth > max_complexity {
        let severity = ctx.get_effective_severity(meta, ty_id);
        if !severity.is_enabled() {
            return;
        }

        ctx.report(
            LintDiagnostic::new(
                NO_COMPLEX_TYPE.id,
                NO_COMPLEX_TYPE.code,
                NO_COMPLEX_TYPE.category,
                severity,
                format!(
                    "type has complexity {} (max {})",
                    visitor.max_depth, max_complexity
                ),
                ctx.module.file_id,
                ctx.tree.get_span(ty_id),
            )
            .with_label("consider using a type alias"),
        );
    }
}

/// Visitor that calculates type nesting complexity.
struct ComplexityVisitor {
    options: NodeVisitorOptions,
    max_depth: usize,
    current_depth: usize,
}

impl NodeVisitor for ComplexityVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            // Generic type application: Array<T>, Map<K, V>
            Expression::Path {
                static_arguments: Some(args),
                ..
            } if !args.is_empty() => {
                self.current_depth += 1;
                self.max_depth = self.max_depth.max(self.current_depth);

                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.current_depth -= 1;
                return;
            }

            // Also check Member with static arguments like foo.Bar<T>
            Expression::Member {
                static_arguments: Some(args),
                ..
            } if !args.is_empty() => {
                self.current_depth += 1;
                self.max_depth = self.max_depth.max(self.current_depth);

                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.current_depth -= 1;
                return;
            }

            // Index types like T[]
            Expression::Index { .. } => {
                self.current_depth += 1;
                self.max_depth = self.max_depth.max(self.current_depth);

                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.current_depth -= 1;
                return;
            }

            // Union types: A | B | C
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr,
                ..
            } => {
                self.current_depth += 1;
                self.max_depth = self.max_depth.max(self.current_depth);

                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.current_depth -= 1;
                return;
            }

            // Intersection types: A & B
            Expression::Binary {
                operator: BinaryOperator::ElementwiseAnd,
                ..
            } => {
                self.current_depth += 1;
                self.max_depth = self.max_depth.max(self.current_depth);

                destack_base::ensure_sufficient_stack(|| {
                    walk_expression(self, tree, id, expression)
                });

                self.current_depth -= 1;
                return;
            }

            _ => {}
        }

        // Default: walk children without incrementing depth
        destack_base::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_complex_type() {
        let test = TestProgram::for_rule_without_builtins(NoComplexType)
            .with_options(|options| options.max_type_complexity = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x: Array<Map<string, List<Set<int32>>>>;
"#,
        );
        test.result(result).assert_lint("no-complex-type");
    }

    #[test]
    fn test_allows_simple_type() {
        let test = TestProgram::for_rule_without_builtins(NoComplexType);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x: Array<string>;
"#,
        );
        test.result(result).assert_no_lint("no-complex-type");
    }

    #[test]
    fn test_allows_moderate_type() {
        let test = TestProgram::for_rule_without_builtins(NoComplexType)
            .with_options(|options| options.max_type_complexity = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x: Map<string, Array<int32>>;
"#,
        );
        test.result(result).assert_no_lint("no-complex-type");
    }

    #[test]
    fn test_counts_union_complexity() {
        let test = TestProgram::for_rule_without_builtins(NoComplexType)
            .with_options(|options| options.max_type_complexity = 2);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x: A | B | (C | D | E);
"#,
        );
        test.result(result).assert_lint("no-complex-type");
    }

    #[test]
    fn test_checks_function_parameter() {
        let test = TestProgram::for_rule_without_builtins(NoComplexType)
            .with_options(|options| options.max_type_complexity = 2);
        let result = test.lint_ast(
            "test.ds",
            r#"
function test(x: Array<Map<string, Set<int32>>>): void {}
"#,
        );
        test.result(result).assert_lint("no-complex-type");
    }
}
