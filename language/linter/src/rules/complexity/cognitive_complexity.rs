use crate::LintMeta;
use destack_dir::{
    self as dir, BinaryOperator, Expression, LocalNodeId, NodeVisitor, NodeVisitorOptions, Tree,
    walk_expression, walk_member, walk_property,
};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, expression_starts_nested_declaration_scope,
    expression_unwrap_statement_source_form, for_each_callable_signature,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit cognitive complexity of functions.
    ///
    /// Cognitive complexity measures how difficult code is to understand.
    /// Nested control flow receives additional penalties.
    #[lint(
        id = "cognitive-complexity",
        code = "LX001",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CognitiveComplexity,
    "Limit cognitive complexity"
}

impl LintRule for CognitiveComplexity {
    fn meta(&self) -> &'static LintMeta {
        CognitiveComplexity::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_complexity = ctx.options.complexity.max_cognitive_complexity;

        // check all callable bodies
        for_each_callable_signature(ctx.dir.tree(), |owner_id, _signature, body_id| {
            // skip declaration only signatures
            let Some(body_id) = body_id else {
                return;
            };

            // compute cognitive complexity for this callable body
            let complexity = compute_callable_cognitive_complexity(ctx.dir.tree(), body_id);
            if complexity <= max_complexity {
                return;
            }

            // report declaration owner violations
            if let CallableOwnerId::Declaration(declaration_id) = owner_id {
                report_cognitive_complexity_violation(
                    ctx,
                    meta,
                    declaration_id,
                    body_id,
                    complexity,
                    max_complexity,
                );
                return;
            }

            // report member owner violations
            if let CallableOwnerId::Member(member_id) = owner_id {
                report_cognitive_complexity_violation(
                    ctx,
                    meta,
                    member_id,
                    body_id,
                    complexity,
                    max_complexity,
                );
                return;
            }

            // report property owner violations
            if let CallableOwnerId::Property(property_id) = owner_id {
                report_cognitive_complexity_violation(
                    ctx,
                    meta,
                    property_id,
                    body_id,
                    complexity,
                    max_complexity,
                );
            }
        });
    }
}

/// Report one cognitive complexity overflow diagnostic.
fn report_cognitive_complexity_violation<T: dir::Node>(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: dir::LocalNodeId<T>,
    body_id: dir::LocalNodeId<dir::Expression>,
    complexity: usize,
    max_complexity: usize,
) {
    // resolve effective severity
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // emit one cognitive complexity overflow diagnostic
    ctx.report(
        LintReport::new(
            COGNITIVE_COMPLEXITY.id,
            COGNITIVE_COMPLEXITY.code,
            COGNITIVE_COMPLEXITY.category,
            severity,
            format!("cognitive complexity {complexity} exceeds maximum of {max_complexity}"),
            ctx.dir.get_span(body_id),
        )
        .label("consider simplifying or extracting logic"),
    );
}

/// Compute cognitive complexity for one callable body.
fn compute_callable_cognitive_complexity(
    tree: &Tree,
    body_expression_id: LocalNodeId<Expression>,
) -> usize {
    // initialize visitor state
    let mut visitor = CognitiveComplexityVisitor {
        options: NodeVisitorOptions::default(),
        root_expression_id: body_expression_id,
        complexity: 0,
        nesting: 0,
    };

    // walk callable body subtree
    let body_expression = tree.get(body_expression_id);
    visitor.visit_expression(tree, body_expression_id, body_expression);

    visitor.complexity
}

/// Logical operator kind tracked for sequence counting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogicalOperatorKind {
    /// `&&`.
    And,
    /// `||`.
    Or,
    /// `??`.
    Coalesce,
}

/// Visitor that computes cognitive complexity for one callable body.
struct CognitiveComplexityVisitor {
    /// Traversal options.
    options: NodeVisitorOptions,
    /// Root callable body expression.
    root_expression_id: LocalNodeId<Expression>,
    /// Accumulated complexity score.
    complexity: usize,
    /// Current nesting level.
    nesting: usize,
}

impl CognitiveComplexityVisitor {
    /// Add structural complexity with nesting penalty.
    fn add_nesting_complexity(&mut self) {
        self.complexity += 1 + self.nesting;
    }

    /// Add structural complexity without nesting penalty.
    fn add_flat_complexity(&mut self) {
        self.complexity += 1;
    }

    /// Return the logical operator kind for one binary operator.
    fn logical_operator_kind(operator: BinaryOperator) -> Option<LogicalOperatorKind> {
        match operator {
            BinaryOperator::And => Some(LogicalOperatorKind::And),
            BinaryOperator::Or => Some(LogicalOperatorKind::Or),
            BinaryOperator::Coalesce => Some(LogicalOperatorKind::Coalesce),
            _ => None,
        }
    }

    /// Return the logical operator kind of one parent expression.
    fn parent_logical_operator_kind(
        &self,
        tree: &Tree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LogicalOperatorKind> {
        // require expression parent node
        let parent_id = tree.get_parent_id(expression_id.id)?;
        if tree.get_node_type(parent_id) != dir::NodeType::Expression {
            return None;
        }

        // resolve parent expression and logical operator
        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        let parent_expression = tree.get(parent_expression_id);
        let dir::Expression::Binary { operator, .. } = parent_expression else {
            return None;
        };

        Self::logical_operator_kind(*operator)
    }
}

impl NodeVisitor for CognitiveComplexityVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &Tree,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // keep nested declaration scopes out of parent callable complexity
        if expression_id != self.root_expression_id {
            let normalized_expression_id =
                expression_unwrap_statement_source_form(tree, expression_id);
            let normalized_expression = tree.get(normalized_expression_id);
            if expression_starts_nested_declaration_scope(normalized_expression) {
                return;
            }
        }

        // handle if and else if chains explicitly
        if let Expression::If {
            condition,
            then_expression,
            else_expression,
            ..
        } = expression
        {
            // every if adds structural and nesting complexity
            self.add_nesting_complexity();

            // visit condition expression at current nesting
            match condition {
                dir::IfCondition::Expression {
                    condition: condition_expression_id,
                } => {
                    let condition_expression = tree.get(*condition_expression_id);
                    self.visit_expression(tree, *condition_expression_id, condition_expression);
                }
                dir::IfCondition::Let { declarator, .. } => {
                    let declarator_id = *declarator;
                    let declarator = tree.get(declarator_id);
                    self.visit_declarator(tree, declarator_id, declarator);
                }
            }

            // then branch receives one nesting level
            self.nesting += 1;
            let then_expression_id = *then_expression;
            let then_expression = tree.get(then_expression_id);
            self.visit_expression(tree, then_expression_id, then_expression);
            self.nesting -= 1;

            // else if stays at this nesting level, plain else adds flat complexity
            if let Some(else_expression_id) = else_expression {
                let else_expression = tree.get(*else_expression_id);
                if matches!(
                    else_expression,
                    Expression::If {
                        form: dir::IfForm::If,
                        ..
                    }
                ) {
                    self.visit_expression(tree, *else_expression_id, else_expression);
                } else {
                    self.add_flat_complexity();
                    self.nesting += 1;
                    self.visit_expression(tree, *else_expression_id, else_expression);
                    self.nesting -= 1;
                }
            }

            return;
        }

        // handle loops and match with nesting penalties
        if matches!(
            expression,
            Expression::While { .. }
                | Expression::For { .. }
                | Expression::ForEach { .. }
                | Expression::Loop { .. }
                | Expression::Match { .. }
        ) {
            self.add_nesting_complexity();
            self.nesting += 1;
            walk_expression(self, tree, expression_id, expression);
            self.nesting -= 1;
            return;
        }

        // handle try catch finally with catch and finally penalties
        if let Expression::Try {
            body,
            catch,
            finally,
            ..
        } = expression
        {
            // visit try body under one nesting level
            self.nesting += 1;
            let body_id = *body;
            let body = tree.get(body_id);
            self.visit_expression(tree, body_id, body);
            self.nesting -= 1;

            // catch receives structural and nesting penalties
            if let Some(catch_id) = catch {
                self.add_nesting_complexity();
                self.nesting += 1;
                let catch = tree.get(*catch_id);
                let catch_expression = tree.get(catch.body);
                self.visit_expression(tree, catch.body, catch_expression);
                self.nesting -= 1;
            }

            // finally receives structural flat penalty
            if let Some(finally_id) = finally {
                self.add_flat_complexity();
                self.nesting += 1;
                let finally = tree.get(*finally_id);
                self.visit_expression(tree, *finally_id, finally);
                self.nesting -= 1;
            }

            return;
        }

        // handle logical operator sequences
        if let Expression::Binary { operator, .. } = expression
            && let Some(operator_kind) = Self::logical_operator_kind(*operator)
            && self.parent_logical_operator_kind(tree, expression_id) != Some(operator_kind)
        {
            self.add_flat_complexity();
        }

        // handle labelled break and continue as structural complexity
        if matches!(
            expression,
            Expression::Break { label: Some(_), .. } | Expression::Continue { label: Some(_) }
        ) {
            self.add_flat_complexity();
        }

        // recurse into expression subtree
        walk_expression(self, tree, expression_id, expression);
    }

    fn visit_property(
        &mut self,
        tree: &Tree,
        property_id: LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        // keep nested object methods out of parent callable complexity
        if matches!(property, dir::Property::Method { .. }) {
            return;
        }

        // recurse into non method properties
        walk_property(self, tree, property_id, property);
    }

    fn visit_member(
        &mut self,
        tree: &Tree,
        member_id: LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        // keep nested member callables out of parent callable complexity
        if matches!(
            member,
            dir::Member::Method { .. }
                | dir::Member::StaticBlock { .. }
                | dir::Member::ComptimeBlock { .. }
        ) {
            return;
        }

        // recurse into non callable members
        walk_member(self, tree, member_id, member);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_high_cognitive_complexity() {
        let test = TestProgram::for_rule_without_prelude(CognitiveComplexity)
            .with_options(|options| options.complexity.max_cognitive_complexity = 5);
        let result = test.lint(
            "cognitive_complexity/test_detects_high_cognitive_complexity.ds",
            r#"
function complex(a: bool, b: bool, c: bool) {
    if (a) {
        if (b) {
            if (c) {
                run()
            }
        }
    } else if (b) {
        run()
    } else {
        run()
    }
}
"#,
        );
        test.result(result).assert_lint("cognitive-complexity");
    }

    #[test]
    fn test_allows_simple_function() {
        let test = TestProgram::for_rule_without_prelude(CognitiveComplexity);
        let result = test.lint(
            "cognitive_complexity/test_allows_simple_function.ds",
            r#"
function simple(x: int32): int32 {
    if (x > 0) {
        return x
    }
    return -x
}
"#,
        );
        test.result(result).assert_no_lint("cognitive-complexity");
    }

    #[test]
    fn test_else_if_chain_does_not_double_nest() {
        let test = TestProgram::for_rule_without_prelude(CognitiveComplexity)
            .with_options(|options| options.complexity.max_cognitive_complexity = 3);
        let result = test.lint(
            "cognitive_complexity/test_else_if_chain_does_not_double_nest.ds",
            r#"
function check(x: int32) {
    if (x == 0) {
        run()
    } else if (x == 1) {
        run()
    } else if (x == 2) {
        run()
    } else {
        run()
    }
}
"#,
        );
        test.result(result).assert_lint("cognitive-complexity");
    }

    #[test]
    fn test_logical_operator_sequences_count_once_per_sequence() {
        let test = TestProgram::for_rule_without_prelude(CognitiveComplexity)
            .with_options(|options| options.complexity.max_cognitive_complexity = 1);
        let result = test.lint(
            "cognitive_complexity/test_logical_operator_sequences_count_once_per_sequence.ds",
            r#"
function check(a: bool, b: bool, c: bool, d: bool): bool {
    return a && b && c || d
}
"#,
        );
        test.result(result).assert_lint("cognitive-complexity");
    }

    #[test]
    fn test_ignores_nested_callable_complexity_for_parent_callable() {
        let test = TestProgram::for_rule_without_prelude(CognitiveComplexity)
            .with_options(|options| options.complexity.max_cognitive_complexity = 1);
        let result = test.lint(
            "cognitive_complexity/test_ignores_nested_callable_complexity_for_parent_callable.ds",
            r#"
function outer() {
    function inner(flag: bool) {
        if (flag) {
            if (flag) {
                run()
            }
        }
    }
    run()
}
"#,
        );
        test.result(result)
            .assert_lint("cognitive-complexity")
            .assert_lint_count("cognitive-complexity", 1);
    }

    #[test]
    fn test_detects_object_method_cognitive_complexity() {
        let test = TestProgram::for_rule_without_prelude(CognitiveComplexity)
            .with_options(|options| options.complexity.max_cognitive_complexity = 2);
        let result = test.lint(
            "cognitive_complexity/test_detects_object_method_cognitive_complexity.ds",
            r#"
const service = {
    run(flag: bool) {
        if (flag) {
            if (flag) {
                process()
            }
        }
    }
}
"#,
        );
        test.result(result).assert_lint("cognitive-complexity");
    }
}
