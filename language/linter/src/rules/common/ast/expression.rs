use destack_ast::{self as ast, BinaryOperator, Block, Expression, Path};
use destack_source::Span;

use crate::LintModuleAstContext;

/// Return whether two expressions are structurally equal.
///
/// Compares expressions by structure, ignoring parentheses.
/// Handles paths, literals, and recursively compares binary/unary operations.
pub fn expressions_equal(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<Expression>,
    right_id: ast::LocalNodeId<Expression>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    // unwrap parentheses
    let left = unwrap_parentheses(ctx, left);
    let right = unwrap_parentheses(ctx, right);
    match (left, right) {
        // paths: compare segments
        (
            Expression::Path {
                path: left_path, ..
            },
            Expression::Path {
                path: right_path, ..
            },
        ) => paths_equal(ctx, left_path, right_path),

        // scalar literals: direct comparison
        (Expression::ScalarLiteral(left_literal), Expression::ScalarLiteral(right_literal)) => {
            left_literal == right_literal
        }

        // type literals: direct comparison
        (Expression::TypeLiteral(left_literal), Expression::TypeLiteral(right_literal)) => {
            left_literal == right_literal
        }

        // binary expressions: compare operator and operands recursively
        (
            Expression::Binary {
                operator: left_operator,
                left: left_left,
                right: left_right,
            },
            Expression::Binary {
                operator: right_operator,
                left: right_left,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && expressions_equal(ctx, *left_left, *right_left)
                && expressions_equal(ctx, *left_right, *right_right)
        }

        // unary expressions: compare operator and operand recursively
        (
            Expression::Unary {
                operator: left_operator,
                right: left_right,
            },
            Expression::Unary {
                operator: right_operator,
                right: right_right,
            },
        ) => left_operator == right_operator && expressions_equal(ctx, *left_right, *right_right),

        // type unary expressions: compare operator and operand
        (
            Expression::TypeUnary {
                operator: left_operator,
                right: left_right,
            },
            Expression::TypeUnary {
                operator: right_operator,
                right: right_right,
            },
        ) => left_operator == right_operator && expressions_equal(ctx, *left_right, *right_right),

        // type binary expressions: compare operator and operands
        (
            Expression::TypeBinary {
                left: left_left,
                operator: left_operator,
                right: left_right,
            },
            Expression::TypeBinary {
                left: right_left,
                operator: right_operator,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && expressions_equal(ctx, *left_left, *right_left)
                && expressions_equal(ctx, *left_right, *right_right)
        }

        // member access: compare object and member name
        (
            Expression::Member {
                left: left_object,
                name: left_name,
                ..
            },
            Expression::Member {
                left: right_object,
                name: right_name,
                ..
            },
        ) => {
            expressions_equal(ctx, *left_object, *right_object)
                && string_ids_equal(ctx, *left_name, *right_name)
        }

        // index access: compare object and index
        (
            Expression::Index {
                left: left_object,
                index: left_index,
                position: left_position,
            },
            Expression::Index {
                left: right_object,
                index: right_index,
                position: right_position,
            },
        ) => {
            if left_position != right_position {
                return false;
            }
            if !expressions_equal(ctx, *left_object, *right_object) {
                return false;
            }
            match (left_index, right_index) {
                (Some(left), Some(right)) => expressions_equal(ctx, *left, *right),
                (None, None) => true,
                _ => false,
            }
        }

        // range expressions: compare start, end, and inclusivity
        (
            Expression::RangeExpression {
                start: left_start,
                end: left_end,
                is_inclusive: left_inclusive,
            },
            Expression::RangeExpression {
                start: right_start,
                end: right_end,
                is_inclusive: right_inclusive,
            },
        ) => {
            left_inclusive == right_inclusive
                && expressions_equal(ctx, *left_start, *right_start)
                && expressions_equal(ctx, *left_end, *right_end)
        }

        // value of: compare mutability, variance, and operand
        (
            Expression::ValueOf {
                mutability: left_mutability,
                variance: left_variance,
                right: left_right,
            },
            Expression::ValueOf {
                mutability: right_mutability,
                variance: right_variance,
                right: right_right,
            },
        ) => {
            left_mutability == right_mutability
                && left_variance == right_variance
                && expressions_equal(ctx, *left_right, *right_right)
        }

        // reference of: compare mutability, variance, and operand
        (
            Expression::ReferenceOf {
                mutability: left_mutability,
                variance: left_variance,
                right: left_right,
            },
            Expression::ReferenceOf {
                mutability: right_mutability,
                variance: right_variance,
                right: right_right,
            },
        ) => {
            left_mutability == right_mutability
                && left_variance == right_variance
                && expressions_equal(ctx, *left_right, *right_right)
        }

        // await expressions: compare inner expression
        (
            Expression::Await {
                expression: left_expression,
            },
            Expression::Await {
                expression: right_expression,
            },
        ) => expressions_equal(ctx, *left_expression, *right_expression),

        // throw expressions: compare value
        (Expression::Throw { value: left_value }, Expression::Throw { value: right_value }) => {
            expressions_equal(ctx, *left_value, *right_value)
        }

        // delete expressions: compare value
        (Expression::Delete { value: left_value }, Expression::Delete { value: right_value }) => {
            expressions_equal(ctx, *left_value, *right_value)
        }

        // maybe expressions: compare position and operand
        (
            Expression::Maybe {
                position: left_position,
                left: left_left,
            },
            Expression::Maybe {
                position: right_position,
                left: right_left,
            },
        ) => left_position == right_position && expressions_equal(ctx, *left_left, *right_left),

        // must expressions: compare position and operand
        (
            Expression::Must {
                position: left_position,
                left: left_left,
            },
            Expression::Must {
                position: right_position,
                left: right_left,
            },
        ) => left_position == right_position && expressions_equal(ctx, *left_left, *right_left),

        // assignment: compare operator and operands
        (
            Expression::Assign {
                left: left_left,
                operator: left_operator,
                right: left_right,
            },
            Expression::Assign {
                left: right_left,
                operator: right_operator,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && expressions_equal(ctx, *left_left, *right_left)
                && expressions_equal(ctx, *left_right, *right_right)
        }

        // call expressions: compare callee and arguments
        (
            Expression::Call {
                left: left_callee,
                dynamic_arguments: left_args,
                ..
            },
            Expression::Call {
                left: right_callee,
                dynamic_arguments: right_args,
                ..
            },
        ) => {
            if !expressions_equal(ctx, *left_callee, *right_callee) {
                return false;
            }
            if left_args.len() != right_args.len() {
                return false;
            }
            for (left_arg_id, right_arg_id) in left_args.iter().zip(right_args.iter()) {
                let left_arg = ctx.tree.get(*left_arg_id);
                let right_arg = ctx.tree.get(*right_arg_id);
                let args_equal = match (left_arg, right_arg) {
                    (
                        ast::Argument::Positional { value: left_value },
                        ast::Argument::Positional { value: right_value },
                    ) => expressions_equal(ctx, *left_value, *right_value),
                    (
                        ast::Argument::Spread { value: left_value },
                        ast::Argument::Spread { value: right_value },
                    ) => expressions_equal(ctx, *left_value, *right_value),
                    _ => false,
                };
                if !args_equal {
                    return false;
                }
            }
            true
        }

        // for other expression types, don't try to compare
        _ => false,
    }
}

/// Unwrap parenthesized expressions to get the inner expression.
fn unwrap_parentheses<'a>(
    ctx: &'a LintModuleAstContext<'_>,
    expression: &'a Expression,
) -> &'a Expression {
    match expression {
        Expression::Parenthesized {
            expression: inner_id,
        } => {
            let inner = ctx.tree.get(*inner_id);
            unwrap_parentheses(ctx, inner)
        }
        _ => expression,
    }
}

/// Return whether two paths are equal.
fn paths_equal(ctx: &LintModuleAstContext<'_>, left: &Path, right: &Path) -> bool {
    if left.segments.len() != right.segments.len() {
        return false;
    }
    for (left_segment, right_segment) in left.segments.iter().zip(right.segments.iter()) {
        if !string_ids_equal(ctx, *left_segment, *right_segment) {
            return false;
        }
    }
    true
}

/// Return whether two string IDs refer to equal strings.
fn string_ids_equal(
    ctx: &LintModuleAstContext<'_>,
    left: ast::StringId,
    right: ast::StringId,
) -> bool {
    let left_string = ctx.strings.get(left);
    let right_string = ctx.strings.get(right);
    left_string.as_ref() == right_string.as_ref()
}

/// Check if an expression has side effects (conservatively returns true if unsure).
///
/// This is useful for lints that want to detect expressions that can be safely removed
/// or that need to distinguish between pure and impure expressions.
pub fn expression_has_side_effects(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        // pure: literals
        Expression::ScalarLiteral(_) | Expression::TypeLiteral(_) => false,

        // pure: paths (variable references)
        Expression::Path { .. } => false,

        // pure: containers (if elements are pure)
        Expression::ArrayExpression { elements } | Expression::TupleExpression { elements } => {
            elements.iter().any(|arg_id| {
                let arg = ctx.tree.get(*arg_id);
                match arg {
                    ast::Argument::Positional { value }
                    | ast::Argument::Spread { value }
                    | ast::Argument::Named { value, .. }
                    | ast::Argument::Labeled { value, .. } => {
                        expression_has_side_effects(ctx, *value)
                    }
                }
            })
        }

        // pure: member access (if object is pure)
        Expression::Member { left, .. } => expression_has_side_effects(ctx, *left),

        // pure: index access (if object and index are pure)
        Expression::Index { left, index, .. } => {
            expression_has_side_effects(ctx, *left)
                || index.is_some_and(|idx| expression_has_side_effects(ctx, idx))
        }

        // pure: unary/binary ops on pure expressions
        Expression::Unary { right, .. } => expression_has_side_effects(ctx, *right),
        Expression::Binary { left, right, .. } => {
            expression_has_side_effects(ctx, *left) || expression_has_side_effects(ctx, *right)
        }

        // pure: type operations
        Expression::TypeUnary { right, .. } => expression_has_side_effects(ctx, *right),
        Expression::TypeBinary { left, right, .. } => {
            expression_has_side_effects(ctx, *left) || expression_has_side_effects(ctx, *right)
        }

        // pure: reference/value of (if operand is pure)
        Expression::ReferenceOf { right, .. } | Expression::ValueOf { right, .. } => {
            expression_has_side_effects(ctx, *right)
        }

        // pure: range (if bounds are pure)
        Expression::RangeExpression { start, end, .. } => {
            expression_has_side_effects(ctx, *start) || expression_has_side_effects(ctx, *end)
        }

        // side effects: calls, assignments, new, await, yield, etc.
        Expression::Call { .. }
        | Expression::Assign { .. }
        | Expression::New { .. }
        | Expression::Await { .. }
        | Expression::Yield { .. }
        | Expression::Delete { .. }
        | Expression::Throw { .. } => true,

        // side effects: control flow
        Expression::Return { .. }
        | Expression::Break { .. }
        | Expression::Continue { .. }
        | Expression::For { .. }
        | Expression::ForEach { .. }
        | Expression::While { .. }
        | Expression::Loop { .. }
        | Expression::If { .. }
        | Expression::Match { .. }
        | Expression::Try { .. } => true,

        // side effects: declarations, imports, exports
        Expression::Declaration(_)
        | Expression::Block(_)
        | Expression::Let { .. }
        | Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::Labelled { .. } => true,

        // side effects: debugger, error, stub
        Expression::Debugger | Expression::Error | Expression::Stub => true,

        // wrapped expressions: check inner
        Expression::Parenthesized { expression } => expression_has_side_effects(ctx, *expression),
        Expression::Statement(inner) => expression_has_side_effects(ctx, *inner),

        // maybe/must propagation: check inner for side effect
        Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
            expression_has_side_effects(ctx, *left)
        }

        // templates: conservatively assume side effects (could have interpolations with calls)
        Expression::TemplateExpression { .. } | Expression::TaggedTemplateExpression { .. } => true,

        // object expressions: check properties for side effects
        Expression::ObjectExpression { .. }
        | Expression::TreeExpression { .. }
        | Expression::SequenceExpression { .. } => true,
    }
}

/// Get the content of a block expression, stripping outer braces if present.
pub fn expression_get_block_span_str(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<Expression>,
) -> String {
    let expr = ctx.tree.get(expr_id);

    // if it's a block, get the content inside the braces
    if let Expression::Block(block_id) = expr {
        let block: &Block = ctx.tree.get(*block_id);
        if let (Some(&first), Some(&last)) = (block.expressions.first(), block.expressions.last()) {
            let first_span = ctx.tree.get_span(first);
            let last_span = ctx.tree.get_span(last);
            let content_span = Span::new(first_span.file, first_span.start, last_span.end);
            return ctx.get_span_text(content_span).to_string();
        }
    }

    // otherwise just return the whole expression text
    ctx.get_span_text(ctx.tree.get_span(expr_id)).to_string()
}

/// Check if an operator is a comparison operator.
pub fn is_comparison_operator(operator: &BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
    )
}

/// Check if an expression is a literal value (scalar or type literal).
pub fn is_literal(expression: &Expression) -> bool {
    match expression {
        Expression::ScalarLiteral(_) => true,
        Expression::TypeLiteral(_) => true,
        Expression::Parenthesized { expression: _ } => {
            // we don't recurse into parenthesized expressions to avoid
            // false positives on complex expressions like (a + b)
            false
        }
        _ => false,
    }
}
