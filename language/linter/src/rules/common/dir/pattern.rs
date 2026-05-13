use destack_dir as dir;

use crate::LintModuleDirContext;

use super::{expression_target_symbol, expression_unwrap_parenthesized};

/// Return true when one DIR pattern matches all remaining values.
pub fn pattern_is_total(tree: &dir::Tree, pattern_id: dir::LocalNodeId<dir::Pattern>) -> bool {
    let pattern = tree.get(pattern_id);

    match pattern {
        dir::Pattern::Wildcard => true,
        dir::Pattern::Binding {
            name: _,
            pattern,
            symbol: _,
        } => pattern
            .map(|inner_pattern_id| pattern_is_total(tree, inner_pattern_id))
            .unwrap_or(true),
        dir::Pattern::Union { patterns } => patterns
            .iter()
            .any(|pattern_id| pattern_is_total(tree, *pattern_id)),
        _ => false,
    }
}

/// Return true when one DIR pattern subsumes another pattern.
pub fn pattern_subsumes_semantically(
    ctx: &mut LintModuleDirContext<'_>,
    left_pattern_id: dir::LocalNodeId<dir::Pattern>,
    right_pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> bool {
    // total patterns subsume every later pattern
    if pattern_is_total(ctx.tree, left_pattern_id) {
        return true;
    }

    // non total patterns cannot subsume total patterns
    if pattern_is_total(ctx.tree, right_pattern_id) {
        return false;
    }

    let left_pattern = ctx.tree.get(left_pattern_id).clone();
    let right_pattern = ctx.tree.get(right_pattern_id).clone();

    // unroll right union branches first
    if let dir::Pattern::Union { patterns } = &right_pattern {
        return patterns
            .iter()
            .all(|pattern_id| pattern_subsumes_semantically(ctx, left_pattern_id, *pattern_id));
    }

    // unroll left union branches next
    if let dir::Pattern::Union { patterns } = &left_pattern {
        return patterns
            .iter()
            .any(|pattern_id| pattern_subsumes_semantically(ctx, *pattern_id, right_pattern_id));
    }

    // unwrap left binding refinements before comparing
    if let dir::Pattern::Binding {
        pattern: Some(inner_pattern_id),
        name: _,
        symbol: _,
    } = &left_pattern
    {
        return pattern_subsumes_semantically(ctx, *inner_pattern_id, right_pattern_id);
    }

    // unwrap right binding refinements before comparing
    if let dir::Pattern::Binding {
        pattern: Some(inner_pattern_id),
        name: _,
        symbol: _,
    } = &right_pattern
    {
        return pattern_subsumes_semantically(ctx, left_pattern_id, *inner_pattern_id);
    }

    match (left_pattern, right_pattern) {
        // compare literal and path expression patterns semantically
        (
            dir::Pattern::Expression {
                value: left_expression_id,
            },
            dir::Pattern::Expression {
                value: right_expression_id,
            },
        ) => pattern_expression_is_equal(ctx, left_expression_id, right_expression_id),

        // compare exact range patterns
        (
            dir::Pattern::Range {
                start: left_start,
                end: left_end,
                end_kind: left_end_kind,
            },
            dir::Pattern::Range {
                start: right_start,
                end: right_end,
                end_kind: right_end_kind,
            },
        ) => {
            left_end_kind == right_end_kind
                && optional_pattern_expression_is_equal(ctx, left_start, right_start)
                && optional_pattern_expression_is_equal(ctx, left_end, right_end)
        }

        // keep wrapper semantics aligned before recursing
        (dir::Pattern::Must(left_inner), dir::Pattern::Must(right_inner)) => {
            pattern_subsumes_semantically(ctx, left_inner, right_inner)
        }
        (
            dir::Pattern::BorrowOf {
                mutability: left_mutability,
                right: left_inner,
            },
            dir::Pattern::BorrowOf {
                mutability: right_mutability,
                right: right_inner,
            },
        ) => {
            left_mutability == right_mutability
                && pattern_subsumes_semantically(ctx, left_inner, right_inner)
        }
        (
            dir::Pattern::MoveOf {
                mutability: left_mutability,
                right: left_inner,
            },
            dir::Pattern::MoveOf {
                mutability: right_mutability,
                right: right_inner,
            },
        ) => {
            left_mutability == right_mutability
                && pattern_subsumes_semantically(ctx, left_inner, right_inner)
        }

        _ => false,
    }
}

/// Return true when two optional DIR pattern expressions are equal.
fn optional_pattern_expression_is_equal(
    ctx: &mut LintModuleDirContext<'_>,
    left: Option<dir::LocalNodeId<dir::Expression>>,
    right: Option<dir::LocalNodeId<dir::Expression>>,
) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => pattern_expression_is_equal(ctx, left, right),
        (None, None) => true,
        _ => false,
    }
}

/// Return true when two pattern expressions are semantically equal.
fn pattern_expression_is_equal(
    ctx: &mut LintModuleDirContext<'_>,
    left_expression_id: dir::LocalNodeId<dir::Expression>,
    right_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let left_expression_id = expression_unwrap_parenthesized(ctx.tree, left_expression_id);
    let right_expression_id = expression_unwrap_parenthesized(ctx.tree, right_expression_id);

    // prefer constant evaluation when both expressions fold
    let left_const_value = ctx.const_value(left_expression_id);
    let right_const_value = ctx.const_value(right_expression_id);
    if left_const_value.is_some() && left_const_value == right_const_value {
        return true;
    }

    // otherwise compare symbol targets
    let left_symbol = expression_target_symbol(ctx, left_expression_id);
    let right_symbol = expression_target_symbol(ctx, right_expression_id);
    if let (Some(left_symbol), Some(right_symbol)) = (left_symbol, right_symbol)
        && left_symbol == right_symbol
    {
        return true;
    }

    let left_expression = ctx.tree.get(left_expression_id);
    let right_expression = ctx.tree.get(right_expression_id);

    match (left_expression, right_expression) {
        (
            dir::Expression::ScalarLiteral { value: left_value },
            dir::Expression::ScalarLiteral { value: right_value },
        ) => left_value == right_value,
        (
            dir::Expression::TypeLiteral { value: left_value },
            dir::Expression::TypeLiteral { value: right_value },
        ) => left_value == right_value,
        _ => false,
    }
}
