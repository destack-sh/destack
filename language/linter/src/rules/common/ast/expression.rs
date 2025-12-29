use destack_ast::{self as ast};

use crate::LintModuleAstContext;

/// Return whether two expressions are structurally equal.
///
/// Compares expressions by structure, ignoring parentheses.
/// Handles paths, literals, and recursively compares binary/unary operations.
pub fn is_equal(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Expression>,
    right_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    // unwrap parentheses
    let left = unwrap_parentheses(ctx, left);
    let right = unwrap_parentheses(ctx, right);
    match (left, right) {
        // paths: compare segments
        (
            ast::Expression::Path {
                path: left_path, ..
            },
            ast::Expression::Path {
                path: right_path, ..
            },
        ) => paths_equal(ctx, left_path, right_path),

        // scalar literals: direct comparison
        (
            ast::Expression::ScalarLiteral(left_literal),
            ast::Expression::ScalarLiteral(right_literal),
        ) => left_literal == right_literal,

        // type literals: direct comparison
        (
            ast::Expression::TypeLiteral(left_literal),
            ast::Expression::TypeLiteral(right_literal),
        ) => left_literal == right_literal,

        // binary expressions: compare operator and operands recursively
        (
            ast::Expression::Binary {
                operator: left_operator,
                left: left_left,
                right: left_right,
            },
            ast::Expression::Binary {
                operator: right_operator,
                left: right_left,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && is_equal(ctx, *left_left, *right_left)
                && is_equal(ctx, *left_right, *right_right)
        }

        // unary expressions: compare operator and operand recursively
        (
            ast::Expression::Unary {
                operator: left_operator,
                right: left_right,
            },
            ast::Expression::Unary {
                operator: right_operator,
                right: right_right,
            },
        ) => left_operator == right_operator && is_equal(ctx, *left_right, *right_right),

        // type unary expressions: compare operator and operand
        (
            ast::Expression::TypeUnary {
                operator: left_operator,
                right: left_right,
            },
            ast::Expression::TypeUnary {
                operator: right_operator,
                right: right_right,
            },
        ) => left_operator == right_operator && is_equal(ctx, *left_right, *right_right),

        // type binary expressions: compare operator and operands
        (
            ast::Expression::TypeBinary {
                left: left_left,
                operator: left_operator,
                right: left_right,
            },
            ast::Expression::TypeBinary {
                left: right_left,
                operator: right_operator,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && is_equal(ctx, *left_left, *right_left)
                && is_equal(ctx, *left_right, *right_right)
        }

        // member access: compare object and member name
        (
            ast::Expression::Member {
                left: left_object,
                name: left_name,
                ..
            },
            ast::Expression::Member {
                left: right_object,
                name: right_name,
                ..
            },
        ) => {
            is_equal(ctx, *left_object, *right_object)
                && string_ids_equal(ctx, *left_name, *right_name)
        }

        // index access: compare object and index
        (
            ast::Expression::Index {
                left: left_object,
                index: left_index,
                position: left_position,
            },
            ast::Expression::Index {
                left: right_object,
                index: right_index,
                position: right_position,
            },
        ) => {
            if left_position != right_position {
                return false;
            }
            if !is_equal(ctx, *left_object, *right_object) {
                return false;
            }
            match (left_index, right_index) {
                (Some(left), Some(right)) => is_equal(ctx, *left, *right),
                (None, None) => true,
                _ => false,
            }
        }

        // range expressions: compare start, end, and inclusivity
        (
            ast::Expression::RangeExpression {
                start: left_start,
                end: left_end,
                is_inclusive: left_inclusive,
            },
            ast::Expression::RangeExpression {
                start: right_start,
                end: right_end,
                is_inclusive: right_inclusive,
            },
        ) => {
            left_inclusive == right_inclusive
                && is_equal(ctx, *left_start, *right_start)
                && is_equal(ctx, *left_end, *right_end)
        }

        // value of: compare mutability, variance, and operand
        (
            ast::Expression::ValueOf {
                mutability: left_mutability,
                variance: left_variance,
                right: left_right,
            },
            ast::Expression::ValueOf {
                mutability: right_mutability,
                variance: right_variance,
                right: right_right,
            },
        ) => {
            left_mutability == right_mutability
                && left_variance == right_variance
                && is_equal(ctx, *left_right, *right_right)
        }

        // reference of: compare mutability, variance, and operand
        (
            ast::Expression::ReferenceOf {
                mutability: left_mutability,
                variance: left_variance,
                right: left_right,
            },
            ast::Expression::ReferenceOf {
                mutability: right_mutability,
                variance: right_variance,
                right: right_right,
            },
        ) => {
            left_mutability == right_mutability
                && left_variance == right_variance
                && is_equal(ctx, *left_right, *right_right)
        }

        // await expressions: compare inner expression
        (
            ast::Expression::Await {
                expression: left_expression,
            },
            ast::Expression::Await {
                expression: right_expression,
            },
        ) => is_equal(ctx, *left_expression, *right_expression),

        // await? expressions: compare inner expression
        (
            ast::Expression::AwaitMaybe {
                expression: left_expression,
            },
            ast::Expression::AwaitMaybe {
                expression: right_expression,
            },
        ) => is_equal(ctx, *left_expression, *right_expression),

        // throw expressions: compare value
        (
            ast::Expression::Throw { value: left_value },
            ast::Expression::Throw { value: right_value },
        ) => is_equal(ctx, *left_value, *right_value),

        // delete expressions: compare value
        (
            ast::Expression::Delete { value: left_value },
            ast::Expression::Delete { value: right_value },
        ) => is_equal(ctx, *left_value, *right_value),

        // maybe expressions: compare position and operand
        (
            ast::Expression::Maybe {
                position: left_position,
                left: left_left,
            },
            ast::Expression::Maybe {
                position: right_position,
                left: right_left,
            },
        ) => left_position == right_position && is_equal(ctx, *left_left, *right_left),

        // must expressions: compare position and operand
        (
            ast::Expression::Must {
                position: left_position,
                left: left_left,
            },
            ast::Expression::Must {
                position: right_position,
                left: right_left,
            },
        ) => left_position == right_position && is_equal(ctx, *left_left, *right_left),

        // assignment: compare operator and operands
        (
            ast::Expression::Assign {
                left: left_left,
                operator: left_operator,
                right: left_right,
            },
            ast::Expression::Assign {
                left: right_left,
                operator: right_operator,
                right: right_right,
            },
        ) => {
            left_operator == right_operator
                && is_equal(ctx, *left_left, *right_left)
                && is_equal(ctx, *left_right, *right_right)
        }

        // call expressions: compare callee and arguments
        (
            ast::Expression::Call {
                left: left_callee,
                dynamic_arguments: left_args,
                ..
            },
            ast::Expression::Call {
                left: right_callee,
                dynamic_arguments: right_args,
                ..
            },
        ) => {
            if !is_equal(ctx, *left_callee, *right_callee) {
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
                        ast::Argument::Positional {
                            value: left_value, ..
                        },
                        ast::Argument::Positional {
                            value: right_value, ..
                        },
                    ) => is_equal(ctx, *left_value, *right_value),
                    (
                        ast::Argument::Spread {
                            value: left_value, ..
                        },
                        ast::Argument::Spread {
                            value: right_value, ..
                        },
                    ) => is_equal(ctx, *left_value, *right_value),
                    _ => false,
                };
                if !args_equal {
                    return false;
                }
            }
            true
        }

        // blocks: compare contents
        (ast::Expression::Block(left_block), ast::Expression::Block(right_block)) => {
            blocks_equal(ctx, *left_block, *right_block)
        }

        // statements: unwrap and compare
        (ast::Expression::Statement(left_inner), ast::Expression::Statement(right_inner)) => {
            is_equal(ctx, *left_inner, *right_inner)
        }
        (ast::Expression::Statement(left_inner), _) => is_equal(ctx, *left_inner, right_id),
        (_, ast::Expression::Statement(right_inner)) => is_equal(ctx, left_id, *right_inner),

        // for other expression types, don't try to compare
        _ => false,
    }
}

/// Check if two blocks have identical expressions.
pub fn blocks_equal(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Block>,
    right_id: ast::LocalNodeId<ast::Block>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);
    if left.expressions.len() != right.expressions.len() {
        return false;
    }
    for (left_expr, right_expr) in left.expressions.iter().zip(right.expressions.iter()) {
        if !is_equal(ctx, *left_expr, *right_expr) {
            return false;
        }
    }
    true
}

/// Unwrap parenthesized expressions to get the inner expression.
fn unwrap_parentheses<'a>(
    ctx: &'a LintModuleAstContext<'_>,
    expression: &'a ast::Expression,
) -> &'a ast::Expression {
    match expression {
        ast::Expression::Parenthesized {
            expression: inner_id,
        } => {
            let inner = ctx.tree.get(*inner_id);
            unwrap_parentheses(ctx, inner)
        }
        _ => expression,
    }
}

/// Return whether two paths are equal.
fn paths_equal(ctx: &LintModuleAstContext<'_>, left: &ast::Path, right: &ast::Path) -> bool {
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
/// #Cleanup: can expression_has_side_effects use NodeVisitor..?
pub fn has_side_effects(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        // pure: literals
        ast::Expression::ScalarLiteral(_) | ast::Expression::TypeLiteral(_) => false,

        // pure: paths (variable references)
        ast::Expression::Path { .. } | ast::Expression::This => false,

        // pure: containers (if elements are pure)
        ast::Expression::ArrayExpression { elements }
        | ast::Expression::TupleExpression { elements } => elements.iter().any(|arg_id| {
            let arg = ctx.tree.get(*arg_id);
            match arg {
                ast::Argument::Positional { value, .. }
                | ast::Argument::Spread { value, .. }
                | ast::Argument::Named { value, .. }
                | ast::Argument::Labeled { value, .. } => has_side_effects(ctx, *value),
            }
        }),

        // pure: member access (if object is pure)
        ast::Expression::Member { left, .. } => has_side_effects(ctx, *left),

        // pure: index access (if object and index are pure)
        ast::Expression::Index { left, index, .. } => {
            has_side_effects(ctx, *left) || index.is_some_and(|idx| has_side_effects(ctx, idx))
        }

        // pure: unary/binary ops on pure expressions
        ast::Expression::Unary { right, .. } => has_side_effects(ctx, *right),
        ast::Expression::Binary { left, right, .. } => {
            has_side_effects(ctx, *left) || has_side_effects(ctx, *right)
        }

        // pure: type operations
        ast::Expression::TypeUnary { right, .. } => has_side_effects(ctx, *right),
        ast::Expression::TypeBinary { left, right, .. } => {
            has_side_effects(ctx, *left) || has_side_effects(ctx, *right)
        }
        ast::Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            has_side_effects(ctx, *left)
                || has_side_effects(ctx, *right)
                || has_side_effects(ctx, *then_type)
                || has_side_effects(ctx, *else_type)
        }
        ast::Expression::TypeMapped {
            parameter, value, ..
        } => {
            has_side_effects(ctx, parameter.constraint)
                || parameter
                    .key_remap
                    .is_some_and(|key_remap| has_side_effects(ctx, key_remap))
                || has_side_effects(ctx, *value)
        }
        ast::Expression::TypeIndex { left, index } => {
            has_side_effects(ctx, *left) || has_side_effects(ctx, *index)
        }
        ast::Expression::TypeTemplateLiteral { spans, .. } => {
            spans.iter().any(|span_id| has_side_effects(ctx, *span_id))
        }
        ast::Expression::TypeImport { .. } => false,
        ast::Expression::TypeInfer { constraint, .. } => {
            constraint.is_some_and(|constraint| has_side_effects(ctx, constraint))
        }
        ast::Expression::TypePredicate { target, .. } => {
            target.is_some_and(|target| has_side_effects(ctx, target))
        }

        // pure: reference/value of (if operand is pure)
        ast::Expression::ReferenceOf { right, .. } | ast::Expression::ValueOf { right, .. } => {
            has_side_effects(ctx, *right)
        }

        // pure: range (if bounds are pure)
        ast::Expression::RangeExpression { start, end, .. } => {
            has_side_effects(ctx, *start) || has_side_effects(ctx, *end)
        }

        // side effects: calls, assignments, new, await, yield, etc.
        ast::Expression::Call { .. }
        | ast::Expression::Assign { .. }
        | ast::Expression::New { .. }
        | ast::Expression::Await { .. }
        | ast::Expression::AwaitMaybe { .. }
        | ast::Expression::Yield { .. }
        | ast::Expression::Delete { .. }
        | ast::Expression::Throw { .. } => true,

        // side effects: control flow
        ast::Expression::Return { .. }
        | ast::Expression::Break { .. }
        | ast::Expression::Continue { .. }
        | ast::Expression::For { .. }
        | ast::Expression::ForEach { .. }
        | ast::Expression::While { .. }
        | ast::Expression::Loop { .. }
        | ast::Expression::If { .. }
        | ast::Expression::Match { .. }
        | ast::Expression::Try { .. } => true,

        // side effects: declarations, imports, exports
        ast::Expression::Declaration(_)
        | ast::Expression::Block(_)
        | ast::Expression::Let { .. }
        | ast::Expression::Using { .. }
        | ast::Expression::Import { .. }
        | ast::Expression::Export { .. }
        | ast::Expression::Labelled { .. } => true,

        // side effects: debugger, error, stub
        ast::Expression::Debugger | ast::Expression::Error | ast::Expression::Stub => true,

        // wrapped expressions: check inner
        ast::Expression::Parenthesized { expression } => has_side_effects(ctx, *expression),
        ast::Expression::Statement(inner) => has_side_effects(ctx, *inner),

        // maybe/must propagation: check inner for side effect
        ast::Expression::Maybe { left, .. } | ast::Expression::Must { left, .. } => {
            has_side_effects(ctx, *left)
        }

        // templates: conservatively assume side effects (could have interpolations with calls)
        ast::Expression::TemplateExpression { .. }
        | ast::Expression::TaggedTemplateExpression { .. } => true,

        // object expressions: check properties for side effects
        ast::Expression::ObjectExpression { .. }
        | ast::Expression::TreeExpression { .. }
        | ast::Expression::SequenceExpression { .. } => true,

        // comptime: check if body has side effects
        ast::Expression::Comptime { body } => has_side_effects(ctx, *body),
    }
}

/// Check if an operator is a comparison operator.
pub fn is_comparison_operator(operator: &ast::BinaryOperator) -> bool {
    matches!(
        operator,
        ast::BinaryOperator::Equal
            | ast::BinaryOperator::NotEqual
            | ast::BinaryOperator::EqualStrict
            | ast::BinaryOperator::NotEqualStrict
            | ast::BinaryOperator::LessThan
            | ast::BinaryOperator::LessThanOrEqual
            | ast::BinaryOperator::GreaterThan
            | ast::BinaryOperator::GreaterThanOrEqual
    )
}

/// Check if an expression is a literal value (scalar or type literal).
pub fn is_literal(expression: &ast::Expression) -> bool {
    matches!(
        expression,
        ast::Expression::ScalarLiteral(_) | ast::Expression::TypeLiteral(_)
    )
}

/// Check if an expression is a constant expression (evaluates to a fixed value at compile time).
pub fn is_constant_expression(ctx: &LintModuleAstContext<'_>, expr: &ast::Expression) -> bool {
    match expr {
        ast::Expression::ScalarLiteral(_) => true,
        ast::Expression::Parenthesized { expression } => {
            is_constant_expression(ctx, ctx.tree.get(*expression))
        }
        ast::Expression::Unary { right, .. } => is_constant_expression(ctx, ctx.tree.get(*right)),
        _ => false,
    }
}

/// Evaluate a constant expression to a boolean value if possible.
///
/// Returns Some(true) for truthy constants, Some(false) for falsy constants, None otherwise.
/// Handles booleans, integers, floats, null/undefined, parenthesized expressions, and unary not.
pub fn constant_to_bool(ctx: &LintModuleAstContext<'_>, expr: &ast::Expression) -> Option<bool> {
    match expr {
        ast::Expression::ScalarLiteral(lit) => match lit {
            ast::ScalarLiteral::Boolean(b) => Some(*b),
            ast::ScalarLiteral::Integer(value) => Some(*value != 0),
            ast::ScalarLiteral::Float(value) => Some(*value != 0.0),
            ast::ScalarLiteral::Bigint(value) => Some(*value != 0),
            _ => None,
        },
        ast::Expression::TypeLiteral(ast::TypeLiteral::Null | ast::TypeLiteral::Undefined) => {
            Some(false)
        }
        ast::Expression::Parenthesized { expression } => {
            constant_to_bool(ctx, ctx.tree.get(*expression))
        }
        ast::Expression::Unary { operator, right } => {
            if *operator == ast::UnaryOperator::Not {
                constant_to_bool(ctx, ctx.tree.get(*right)).map(|b| !b)
            } else {
                None
            }
        }
        _ => None,
    }
}
