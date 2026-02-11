use std::cmp::Ordering;

use destack_ast as ast;

use super::expression_is_equal;
use crate::{ConstValue, LintModuleAstContext};

/// Return the selector for one match case.
pub fn match_case_selector(case: &ast::MatchCase) -> &ast::MatchSelector {
    match case {
        ast::MatchCase::Expression { selector, .. } => selector,
        ast::MatchCase::Block { selector, .. } => selector,
    }
}

/// Return true when a selector is the default case.
pub fn match_selector_is_default(selector: &ast::MatchSelector) -> bool {
    matches!(selector, ast::MatchSelector::Default)
}

/// Return true when a selector has a guard expression.
pub fn match_selector_has_guard(selector: &ast::MatchSelector) -> bool {
    matches!(selector, ast::MatchSelector::Pattern { guard: Some(_), .. })
}

/// Return the pattern id for a pattern selector.
pub fn match_selector_pattern_id(
    selector: &ast::MatchSelector,
) -> Option<ast::LocalNodeId<ast::Pattern>> {
    match selector {
        ast::MatchSelector::Pattern { pattern, .. } => Some(*pattern),
        ast::MatchSelector::Default => None,
    }
}

/// Return the expression id when a selector is an expression pattern.
pub fn match_selector_expression_id(
    ctx: &LintModuleAstContext<'_>,
    selector: &ast::MatchSelector,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let pattern_id = match_selector_pattern_id(selector)?;
    pattern_expression_id(ctx, pattern_id)
}

/// Return the expression id when a pattern wraps one expression pattern.
pub fn pattern_expression_id(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Expression { value } => Some(*value),
        ast::Pattern::Binding {
            pattern: Some(inner_pattern_id),
            ..
        }
        | ast::Pattern::Must(inner_pattern_id)
        | ast::Pattern::ReferenceOf {
            right: inner_pattern_id,
            ..
        }
        | ast::Pattern::ValueOf {
            right: inner_pattern_id,
            ..
        } => pattern_expression_id(ctx, *inner_pattern_id),
        _ => None,
    }
}

/// Return true when a pattern matches all values.
pub fn pattern_matches_all(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> bool {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Wildcard => true,
        ast::Pattern::Binding { pattern, .. } => pattern
            .map(|inner_pattern_id| pattern_matches_all(ctx, inner_pattern_id))
            .unwrap_or(true),
        ast::Pattern::Union { patterns } => patterns
            .iter()
            .any(|pattern_id| pattern_matches_all(ctx, *pattern_id)),
        _ => false,
    }
}

/// Return true when one pattern subsumes another pattern.
pub fn pattern_subsumes(
    ctx: &mut LintModuleAstContext<'_>,
    left_pattern_id: ast::LocalNodeId<ast::Pattern>,
    right_pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> bool {
    // patterns that match everything subsume all later patterns
    if pattern_matches_all(ctx, left_pattern_id) {
        return true;
    }

    // non-total patterns cannot subsume total patterns
    if pattern_matches_all(ctx, right_pattern_id) {
        return false;
    }

    let left_pattern = ctx.tree.get(left_pattern_id);
    let right_pattern = ctx.tree.get(right_pattern_id);

    // unroll right union: each branch must be subsumed
    if let ast::Pattern::Union { patterns } = right_pattern {
        return patterns
            .iter()
            .all(|pattern_id| pattern_subsumes(ctx, left_pattern_id, *pattern_id));
    }

    // unroll left union: any branch can subsume
    if let ast::Pattern::Union { patterns } = left_pattern {
        return patterns
            .iter()
            .any(|pattern_id| pattern_subsumes(ctx, *pattern_id, right_pattern_id));
    }

    // binding with inner pattern inherits inner coverage
    if let ast::Pattern::Binding {
        pattern: Some(inner_pattern_id),
        ..
    } = left_pattern
    {
        return pattern_subsumes(ctx, *inner_pattern_id, right_pattern_id);
    }

    // binding with inner pattern on the right unwraps before comparison
    if let ast::Pattern::Binding {
        pattern: Some(inner_pattern_id),
        ..
    } = right_pattern
    {
        return pattern_subsumes(ctx, left_pattern_id, *inner_pattern_id);
    }

    match (left_pattern, right_pattern) {
        // exact literal equality
        (
            ast::Pattern::Expression {
                value: left_expression_id,
            },
            ast::Pattern::Expression {
                value: right_expression_id,
            },
        ) => expression_is_equal(ctx, *left_expression_id, *right_expression_id),

        // range contains literal
        (
            ast::Pattern::Range {
                start: left_start,
                end: left_end,
                is_inclusive: left_is_inclusive,
            },
            ast::Pattern::Expression {
                value: right_expression_id,
            },
        ) => range_contains_expression(
            ctx,
            *left_start,
            *left_end,
            *left_is_inclusive,
            *right_expression_id,
        ),

        // range contains range
        (
            ast::Pattern::Range {
                start: left_start,
                end: left_end,
                is_inclusive: left_is_inclusive,
            },
            ast::Pattern::Range {
                start: right_start,
                end: right_end,
                is_inclusive: right_is_inclusive,
            },
        ) => range_contains_range(
            ctx,
            *left_start,
            *left_end,
            *left_is_inclusive,
            *right_start,
            *right_end,
            *right_is_inclusive,
        ),

        // must wrappers are comparable only when wrapper shape matches
        (ast::Pattern::Must(left_inner), ast::Pattern::Must(right_inner)) => {
            pattern_subsumes(ctx, *left_inner, *right_inner)
        }

        // reference wrappers are comparable only with equal mutability
        (
            ast::Pattern::ReferenceOf {
                mutability: left_mutability,
                right: left_inner,
            },
            ast::Pattern::ReferenceOf {
                mutability: right_mutability,
                right: right_inner,
            },
        ) => {
            left_mutability == right_mutability && pattern_subsumes(ctx, *left_inner, *right_inner)
        }

        // value wrappers are comparable only with equal mutability
        (
            ast::Pattern::ValueOf {
                mutability: left_mutability,
                right: left_inner,
            },
            ast::Pattern::ValueOf {
                mutability: right_mutability,
                right: right_inner,
            },
        ) => {
            left_mutability == right_mutability && pattern_subsumes(ctx, *left_inner, *right_inner)
        }

        _ => false,
    }
}

/// One scalar pattern value used for range and literal checks.
#[derive(Debug, Clone, Copy)]
enum PatternScalar {
    /// Boolean scalar value.
    Boolean(bool),
    /// Integer scalar value.
    Integer(i64),
    /// Bigint scalar value.
    Bigint(i64),
    /// Float scalar value.
    Float(f64),
    /// Character scalar value.
    Character(char),
}

/// Return true when one range contains one literal expression.
fn range_contains_expression(
    ctx: &mut LintModuleAstContext<'_>,
    range_start: Option<ast::LocalNodeId<ast::Pattern>>,
    range_end: Option<ast::LocalNodeId<ast::Pattern>>,
    is_end_inclusive: bool,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let Some(value) = expression_scalar_value(ctx, expression_id) else {
        return false;
    };

    range_matches_scalar(ctx, range_start, range_end, is_end_inclusive, value)
}

/// Return true when one range fully contains another range.
fn range_contains_range(
    ctx: &mut LintModuleAstContext<'_>,
    left_start: Option<ast::LocalNodeId<ast::Pattern>>,
    left_end: Option<ast::LocalNodeId<ast::Pattern>>,
    left_end_inclusive: bool,
    right_start: Option<ast::LocalNodeId<ast::Pattern>>,
    right_end: Option<ast::LocalNodeId<ast::Pattern>>,
    right_end_inclusive: bool,
) -> bool {
    // compare lower bounds
    let lower_bounds_ok = match (left_start, right_start) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(left_start), Some(right_start)) => {
            let Some(left_start_value) = pattern_scalar_value(ctx, left_start) else {
                return false;
            };
            let Some(right_start_value) = pattern_scalar_value(ctx, right_start) else {
                return false;
            };

            matches!(
                scalar_compare(left_start_value, right_start_value),
                Some(Ordering::Less | Ordering::Equal)
            )
        }
    };
    if !lower_bounds_ok {
        return false;
    }

    // compare upper bounds
    match (left_end, right_end) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(left_end), Some(right_end)) => {
            let Some(left_end_value) = pattern_scalar_value(ctx, left_end) else {
                return false;
            };
            let Some(right_end_value) = pattern_scalar_value(ctx, right_end) else {
                return false;
            };

            match scalar_compare(left_end_value, right_end_value) {
                Some(Ordering::Greater) => true,
                Some(Ordering::Less) => false,
                Some(Ordering::Equal) => left_end_inclusive || !right_end_inclusive,
                None => false,
            }
        }
    }
}

/// Return true when one scalar value is contained in a range.
fn range_matches_scalar(
    ctx: &LintModuleAstContext<'_>,
    range_start: Option<ast::LocalNodeId<ast::Pattern>>,
    range_end: Option<ast::LocalNodeId<ast::Pattern>>,
    is_end_inclusive: bool,
    value: PatternScalar,
) -> bool {
    // check lower bound
    if let Some(start_pattern_id) = range_start {
        let Some(start_value) = pattern_scalar_value(ctx, start_pattern_id) else {
            return false;
        };
        let Some(ordering) = scalar_compare(value, start_value) else {
            return false;
        };
        if ordering == Ordering::Less {
            return false;
        }
    }

    // check upper bound
    if let Some(end_pattern_id) = range_end {
        let Some(end_value) = pattern_scalar_value(ctx, end_pattern_id) else {
            return false;
        };
        let Some(ordering) = scalar_compare(value, end_value) else {
            return false;
        };

        if ordering == Ordering::Greater {
            return false;
        }
        if ordering == Ordering::Equal && !is_end_inclusive {
            return false;
        }
    }

    true
}

/// Return one scalar value represented by one expression.
fn expression_scalar_value(
    ctx: &mut LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<PatternScalar> {
    // preserve char literals which are not part of ConstValue
    if let ast::Expression::ScalarLiteral(ast::ScalarLiteral::Character(value)) =
        ctx.tree.get(expression_id)
    {
        return Some(PatternScalar::Character(*value));
    }

    // reuse shared constant evaluation for scalar values
    let const_value = ctx.const_value(expression_id)?;
    const_value_to_pattern_scalar(const_value)
}

/// Return one scalar value represented by one pattern expression.
fn pattern_scalar_value(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<PatternScalar> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Binding {
            pattern: Some(inner_pattern_id),
            ..
        }
        | ast::Pattern::Must(inner_pattern_id)
        | ast::Pattern::ReferenceOf {
            right: inner_pattern_id,
            ..
        }
        | ast::Pattern::ValueOf {
            right: inner_pattern_id,
            ..
        } => pattern_scalar_value(ctx, *inner_pattern_id),
        ast::Pattern::Expression { value } => expression_scalar_value_readonly(ctx, *value),
        _ => None,
    }
}

/// Return one scalar value represented by one expression in readonly mode.
fn expression_scalar_value_readonly(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<PatternScalar> {
    let expression = ctx.tree.get(expression_id);
    match expression {
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::Boolean(value)) => {
            Some(PatternScalar::Boolean(*value))
        }
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::Integer(value)) => {
            Some(PatternScalar::Integer(*value))
        }
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::Bigint(value)) => {
            Some(PatternScalar::Bigint(*value))
        }
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::Float(value)) => {
            Some(PatternScalar::Float(*value))
        }
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::Character(value)) => {
            Some(PatternScalar::Character(*value))
        }
        ast::Expression::Parenthesized { expression } => {
            expression_scalar_value_readonly(ctx, *expression)
        }
        _ => None,
    }
}

/// Convert one constant value into one scalar pattern value.
fn const_value_to_pattern_scalar(const_value: ConstValue) -> Option<PatternScalar> {
    match const_value {
        ConstValue::Boolean(value) => Some(PatternScalar::Boolean(value)),
        ConstValue::Integer(value) => Some(PatternScalar::Integer(value)),
        ConstValue::Bigint(value) => Some(PatternScalar::Bigint(value)),
        ConstValue::Float(value) => Some(PatternScalar::Float(value)),
        ConstValue::Null | ConstValue::Undefined => None,
    }
}

/// Compare two scalar pattern values.
fn scalar_compare(left: PatternScalar, right: PatternScalar) -> Option<Ordering> {
    match (left, right) {
        (PatternScalar::Boolean(left), PatternScalar::Boolean(right)) => Some(left.cmp(&right)),
        (PatternScalar::Integer(left), PatternScalar::Integer(right)) => Some(left.cmp(&right)),
        (PatternScalar::Bigint(left), PatternScalar::Bigint(right)) => Some(left.cmp(&right)),
        (PatternScalar::Integer(left), PatternScalar::Bigint(right)) => Some(left.cmp(&right)),
        (PatternScalar::Bigint(left), PatternScalar::Integer(right)) => Some(left.cmp(&right)),
        (PatternScalar::Float(left), PatternScalar::Float(right)) => left.partial_cmp(&right),
        (PatternScalar::Character(left), PatternScalar::Character(right)) => Some(left.cmp(&right)),
        _ => None,
    }
}
