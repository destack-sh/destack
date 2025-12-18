use destack_ast::{self as ast, Expression, Path};

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
