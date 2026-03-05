use destack_ast::{
    self as ast, BinaryOperator, Expression, LocalNodeId, NodeTree, NodeVisitor,
    NodeVisitorOptions, walk_expression,
};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_is_type_annotation;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Warn on overly complex type expressions.
    ///
    /// Deeply nested generic types and large type compositions are hard to read.
    /// Consider introducing named type aliases for complex shapes.
    #[lint(
        id = "no-complex-type",
        code = "LX016",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
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
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_type_complexity = ctx.options.max_type_complexity;

        // check only top level type annotation roots
        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            if !expression_is_type_annotation(ctx.tree, &ctx.parents, expression_id) {
                continue;
            }
            if has_type_annotation_expression_parent(ctx, expression_id) {
                continue;
            }

            // compute structural complexity score for this type expression
            let complexity = type_expression_complexity(ctx.tree, expression_id);
            if complexity <= max_type_complexity {
                continue;
            }

            // resolve effective severity
            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // report one complex type diagnostic
            ctx.report(
                LintDiagnostic::new(
                    NO_COMPLEX_TYPE.id,
                    NO_COMPLEX_TYPE.code,
                    NO_COMPLEX_TYPE.category,
                    severity,
                    format!("type has complexity {complexity} (max {max_type_complexity})"),
                    ctx.module.file_id,
                    ctx.tree.get_span(expression_id),
                )
                .with_label("consider extracting a named type alias"),
            );
        }
    }
}

/// Return true when one type expression has a parent type expression.
fn has_type_annotation_expression_parent(
    ctx: &LintModuleAstContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    // resolve expression parent node
    let Some(parent_id) = ctx.parents.get(expression_id) else {
        return false;
    };
    if ctx.tree.get_node_type(parent_id) != ast::NodeType::Expression {
        return false;
    }

    // keep only parents that also live in type annotation positions
    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    expression_is_type_annotation(ctx.tree, &ctx.parents, parent_expression_id)
}

/// Compute one nesting style complexity score for a type expression.
fn type_expression_complexity(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> usize {
    // initialize complexity visitor state
    let mut visitor = TypeComplexityVisitor {
        options: NodeVisitorOptions::default(),
        current_depth: 0,
        max_depth: 0,
    };

    // walk type expression subtree
    let expression = tree.get(expression_id);
    visitor.visit_expression(tree, expression_id, expression);

    visitor.max_depth
}

/// Visitor that tracks type nesting complexity depth.
struct TypeComplexityVisitor {
    /// Traversal options.
    options: NodeVisitorOptions,
    /// Current nesting depth.
    current_depth: usize,
    /// Maximum observed nesting depth.
    max_depth: usize,
}

impl TypeComplexityVisitor {
    /// Enter one complexity increasing node.
    fn enter_complexity_node(&mut self) {
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
    }

    /// Leave one complexity increasing node.
    fn leave_complexity_node(&mut self) {
        self.current_depth -= 1;
    }
}

impl NodeVisitor for TypeComplexityVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // track expressions that increase type nesting depth
        let increases_depth = matches!(
            expression,
            Expression::Path {
                static_arguments: Some(arguments),
                ..
            } if !arguments.is_empty()
        ) || matches!(
            expression,
            Expression::Member {
                static_arguments: Some(arguments),
                ..
            } if !arguments.is_empty()
        ) || matches!(
            expression,
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                ..
            }
        ) || matches!(
            expression,
            Expression::TypeBinary { .. }
                | Expression::TypeUnary { .. }
                | Expression::TypeConditional { .. }
                | Expression::TypeMapped { .. }
                | Expression::TypeIndex { .. }
                | Expression::TypeTemplateLiteral { .. }
                | Expression::TupleExpression { .. }
                | Expression::ObjectExpression { .. }
                | Expression::Index { .. }
        );

        // apply nested depth accounting around child traversal
        if increases_depth {
            self.enter_complexity_node();
            walk_expression(self, tree, expression_id, expression);
            self.leave_complexity_node();
            return;
        }

        // recurse for non complexity increasing nodes
        walk_expression(self, tree, expression_id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_complex_type_alias() {
        let test = TestProgram::for_rule_without_prelude(NoComplexType)
            .with_options(|options| options.max_type_complexity = 3);
        let result = test.lint_ast(
            "no_complex_type/test_detects_complex_type_alias.ds",
            r#"
type Value = Array<Map<string, List<Set<int32>>>>;
"#,
        );
        test.result(result).assert_lint("no-complex-type");
    }

    #[test]
    fn test_allows_simple_type_annotation() {
        let test = TestProgram::for_rule_without_prelude(NoComplexType);
        let result = test.lint_ast(
            "no_complex_type/test_allows_simple_type_annotation.ds",
            r#"
let value: Array<string>;
"#,
        );
        test.result(result).assert_no_lint("no-complex-type");
    }

    #[test]
    fn test_detects_complex_parameter_type() {
        let test = TestProgram::for_rule_without_prelude(NoComplexType)
            .with_options(|options| options.max_type_complexity = 2);
        let result = test.lint_ast(
            "no_complex_type/test_detects_complex_parameter_type.ds",
            r#"
function run(value: Array<Map<string, Set<int32>>>): void {}
"#,
        );
        test.result(result).assert_lint("no-complex-type");
    }

    #[test]
    fn test_ignores_runtime_expression_complexity() {
        let test = TestProgram::for_rule_without_prelude(NoComplexType)
            .with_options(|options| options.max_type_complexity = 1);
        let result = test.lint_ast(
            "no_complex_type/test_ignores_runtime_expression_complexity.ds",
            r#"
let value = a | b | c | d;
"#,
        );
        test.result(result).assert_no_lint("no-complex-type");
    }
}
