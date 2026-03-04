use destack_dir::{self as dir, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `void` expressions where a value is expected.
    ///
    /// `void` is useful as a top-level statement when intentionally discarding
    /// a result.
    /// Nested `void` expressions inside value-producing positions are often
    /// confusing and harder to read.
    #[lint(
        id = "no-confusing-void-expression",
        code = "LC068",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoConfusingVoidExpression,
    "Disallow confusing nested void expressions"
}

impl LintRule for NoConfusingVoidExpression {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoConfusingVoidExpression::meta()
    }

    /// Check module DIR nodes for nested `void` expressions.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            if !is_void_unary_expression(ctx.tree, expression_id) {
                continue;
            }

            if is_statement_void_discard(ctx.tree, expression_id) {
                continue;
            }
            if is_non_tail_sequence_operand(ctx.tree, expression_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.get_span(expression_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_CONFUSING_VOID_EXPRESSION.id,
                    NO_CONFUSING_VOID_EXPRESSION.code,
                    NO_CONFUSING_VOID_EXPRESSION.category,
                    severity,
                    "confusing void expression in value position",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use `void` as a standalone statement or refactor this expression"),
            );
        }
    }
}

/// Return true when this void expression is a non-tail sequence operand.
fn is_non_tail_sequence_operand(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let outer_expression_id = outer_passthrough_expression_id(tree, expression_id);
    let Some(parent_id) = parent_expression_id(tree, outer_expression_id) else {
        return false;
    };
    let parent_expression = tree.get(parent_id);
    let dir::Expression::SequenceExpression { expressions } = parent_expression else {
        return false;
    };

    expressions
        .last()
        .is_some_and(|last_expression_id| *last_expression_id != outer_expression_id)
}

/// Return true when the expression is a `void` unary expression.
fn is_void_unary_expression(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    matches!(
        tree.get(expression_id),
        dir::Expression::Unary {
            operator: UnaryOperator::Void,
            ..
        }
    )
}

/// Return true when this `void` expression is used as a standalone statement discard.
fn is_statement_void_discard(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let outer_expression_id = outer_passthrough_expression_id(tree, expression_id);
    let Some(parent_expression_id) = parent_expression_id(tree, outer_expression_id) else {
        return false;
    };

    let parent_expression = tree.get(parent_expression_id);
    matches!(
        parent_expression,
        dir::Expression::Statement { statement } if *statement == outer_expression_id
    )
}

/// Return the outermost passthrough expression that still wraps this node.
fn outer_passthrough_expression_id(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    let mut current_expression_id = expression_id;

    loop {
        let Some(parent_expression_id) = parent_expression_id(tree, current_expression_id) else {
            return current_expression_id;
        };
        let parent_expression = tree.get(parent_expression_id);
        if !is_passthrough_parent(parent_expression, current_expression_id) {
            return current_expression_id;
        }

        current_expression_id = parent_expression_id;
    }
}

/// Return one parent expression id when the parent node is an expression.
fn parent_expression_id(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let parent_id = tree.get_parent(expression_id.id)?;
    if parent_id.ty != dir::NodeType::Expression {
        return None;
    }

    Some(parent_id.into_typed::<dir::Expression>())
}

/// Return true when this parent expression is a transparent wrapper for the child expression.
fn is_passthrough_parent(
    parent_expression: &dir::Expression,
    child_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    matches!(
        parent_expression,
        dir::Expression::Parenthesized { expression } if *expression == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::Maybe { left } if *left == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::Must { left } if *left == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::Cast { value, .. } if *value == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::OwnershipCast { value, .. } if *value == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::ValueOf { right, .. } if *right == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::ReferenceOf { right, .. } if *right == child_expression_id
    ) || matches!(
        parent_expression,
        dir::Expression::PointerOf { right, .. } if *right == child_expression_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Allow standalone `void` discard statements.
    #[test]
    fn test_allows_standalone_void_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_standalone_void_statement.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

void sideEffect();
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Allow parenthesized standalone `void` discard statements.
    #[test]
    fn test_allows_parenthesized_void_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_parenthesized_void_statement.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

(void sideEffect());
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Flag `void` used in variable initialization.
    #[test]
    fn test_flags_void_in_variable_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_in_variable_initializer.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = void sideEffect();
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag `void` used in return values.
    #[test]
    fn test_flags_void_in_return_value() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_in_return_value.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

function run(): unknown {
    return void sideEffect();
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag `void` used as a function argument.
    #[test]
    fn test_flags_void_in_argument_position() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_in_argument_position.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

function consume(value: unknown): void {
    value;
}

consume(void sideEffect());
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Allow void in non-tail sequence positions.
    #[test]
    fn test_allows_void_in_non_tail_sequence() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_allows_void_in_non_tail_sequence.ts",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = (void sideEffect(), 1);
"#,
        );
        test.result(result)
            .assert_no_lint("no-confusing-void-expression");
    }

    /// Flag void in tail sequence positions.
    #[test]
    fn test_flags_void_in_tail_sequence() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_in_tail_sequence.ts",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = (1, void sideEffect());
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag `void` when nested under passthrough wrappers in value position.
    #[test]
    fn test_flags_void_under_parenthesized_return_wrapper() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_void_under_parenthesized_return_wrapper.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

function run(): unknown {
    return (void sideEffect());
}
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag cast-wrapped `void` statements because the cast wrapper is noisy.
    #[test]
    fn test_flags_cast_wrapped_void_statement() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_cast_wrapped_void_statement.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

((void sideEffect()) as unknown);
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }

    /// Flag cast-wrapped `void` expressions in value-producing positions.
    #[test]
    fn test_flags_cast_wrapped_void_in_initializer() {
        let test = TestProgram::for_rule_without_prelude(NoConfusingVoidExpression);
        let result = test.lint_dir(
            "no_confusing_void_expression/test_flags_cast_wrapped_void_in_initializer.ds",
            r#"
function sideEffect(): unknown {
    return 1;
}

const value = (void sideEffect()) as unknown;
"#,
        );
        test.result(result)
            .assert_lint("no-confusing-void-expression");
    }
}
