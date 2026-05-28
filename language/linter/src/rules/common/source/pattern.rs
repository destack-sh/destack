use destack_dir as dir;

use super::expression_is_equal;
use crate::LintModuleContext;

/// Return the expression id when a selector is an expression pattern.
pub fn match_selector_expression_id(
    ctx: &LintModuleContext<'_>,
    selector: &dir::MatchSelector,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let pattern_id = selector.pattern_id()?;
    pattern_expression_id(ctx, pattern_id)
}

/// Return one default expression id for a parameter when present.
pub fn parameter_default_expression_id(
    parameter: &dir::Parameter,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match parameter {
        dir::Parameter::Named { default, .. } | dir::Parameter::Pattern { default, .. } => *default,
        dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => None,
        dir::Parameter::Error => None,
    }
}

/// Return one default expression id for a pattern field when present.
pub fn pattern_field_default_expression_id(
    tree: &dir::Tree,
    pattern_field: &dir::PatternField,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match pattern_field {
        dir::PatternField::Named { pattern, .. } => {
            pattern.and_then(|pattern_id| pattern_assignment_value_expression_id(tree, pattern_id))
        }
        dir::PatternField::Computed { pattern, .. } => {
            pattern_assignment_value_expression_id(tree, *pattern)
        }
        dir::PatternField::Positional { pattern } => {
            pattern_assignment_value_expression_id(tree, *pattern)
        }
        dir::PatternField::Spread { .. } | dir::PatternField::Elision => None,
    }
}

/// Return the default value expression for one assignment pattern.
fn pattern_assignment_value_expression_id(
    tree: &dir::Tree,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match tree.get(pattern_id) {
        dir::Pattern::Assign { value, .. } => Some(*value),
        dir::Pattern::Binding {
            pattern: Some(inner_pattern_id),
            ..
        }
        | dir::Pattern::Must(inner_pattern_id)
        | dir::Pattern::BorrowOf {
            right: inner_pattern_id,
            ..
        }
        | dir::Pattern::MoveOf {
            right: inner_pattern_id,
            ..
        }
        | dir::Pattern::DereferenceOf {
            right: inner_pattern_id,
        } => pattern_assignment_value_expression_id(tree, *inner_pattern_id),
        dir::Pattern::Wildcard
        | dir::Pattern::Binding { pattern: None, .. }
        | dir::Pattern::Expression { .. }
        | dir::Pattern::Range { .. }
        | dir::Pattern::TypeExpression { .. }
        | dir::Pattern::Tuple { .. }
        | dir::Pattern::Newtype { .. }
        | dir::Pattern::Sequence { .. }
        | dir::Pattern::Object { .. }
        | dir::Pattern::NominalObject { .. }
        | dir::Pattern::Union { .. } => None,
    }
}

/// Return the expression id when a pattern wraps one expression pattern.
pub fn pattern_expression_id(
    ctx: &LintModuleContext<'_>,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let pattern = ctx.dir.get(pattern_id);
    match pattern {
        dir::Pattern::Expression { value } => Some(*value),
        dir::Pattern::Assign { pattern, .. } => pattern_expression_id(ctx, *pattern),
        dir::Pattern::Binding {
            pattern: Some(inner_pattern_id),
            ..
        }
        | dir::Pattern::Must(inner_pattern_id)
        | dir::Pattern::BorrowOf {
            right: inner_pattern_id,
            ..
        }
        | dir::Pattern::MoveOf {
            right: inner_pattern_id,
            ..
        }
        | dir::Pattern::DereferenceOf {
            right: inner_pattern_id,
        } => pattern_expression_id(ctx, *inner_pattern_id),
        _ => None,
    }
}

/// Return true when a pattern matches all values.
pub fn pattern_matches_all(
    ctx: &LintModuleContext<'_>,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> bool {
    let pattern = ctx.dir.get(pattern_id);
    match pattern {
        dir::Pattern::Wildcard => true,
        dir::Pattern::Assign { pattern, .. } => pattern_matches_all(ctx, *pattern),
        dir::Pattern::Binding { pattern, .. } => pattern
            .map(|inner_pattern_id| pattern_matches_all(ctx, inner_pattern_id))
            .unwrap_or(true),
        dir::Pattern::Union { patterns } => patterns
            .iter()
            .any(|pattern_id| pattern_matches_all(ctx, *pattern_id)),
        _ => false,
    }
}

/// Return true when one pattern is `_` or one underscore prefixed binding.
pub fn pattern_is_underscore_binding_or_wildcard(
    ctx: &LintModuleContext<'_>,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> bool {
    // resolve one pattern node
    let pattern = ctx.dir.get(pattern_id);

    // match wildcard and underscore bindings
    match pattern {
        dir::Pattern::Wildcard => true,
        dir::Pattern::Assign { pattern, .. } => {
            pattern_is_underscore_binding_or_wildcard(ctx, *pattern)
        }
        dir::Pattern::Binding { name, .. } => ctx.strings.get(*name).starts_with('_'),
        _ => false,
    }
}

/// Return true when one pattern subsumes another pattern.
pub fn pattern_subsumes(
    ctx: &mut LintModuleContext<'_>,
    left_pattern_id: dir::LocalNodeId<dir::Pattern>,
    right_pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> bool {
    // patterns that match everything subsume all later patterns
    if pattern_matches_all(ctx, left_pattern_id) {
        return true;
    }

    // non-total patterns cannot subsume total patterns
    if pattern_matches_all(ctx, right_pattern_id) {
        return false;
    }

    let left_pattern = ctx.dir.get(left_pattern_id);
    let right_pattern = ctx.dir.get(right_pattern_id);

    // unroll right union: each branch must be subsumed
    if let dir::Pattern::Union { patterns } = right_pattern {
        return patterns
            .iter()
            .all(|pattern_id| pattern_subsumes(ctx, left_pattern_id, *pattern_id));
    }

    // unroll left union: any branch can subsume
    if let dir::Pattern::Union { patterns } = left_pattern {
        return patterns
            .iter()
            .any(|pattern_id| pattern_subsumes(ctx, *pattern_id, right_pattern_id));
    }

    // binding with inner pattern inherits inner coverage
    if let dir::Pattern::Binding {
        pattern: Some(inner_pattern_id),
        ..
    } = left_pattern
    {
        return pattern_subsumes(ctx, *inner_pattern_id, right_pattern_id);
    }

    // binding with inner pattern on the right unwraps before comparison
    if let dir::Pattern::Binding {
        pattern: Some(inner_pattern_id),
        ..
    } = right_pattern
    {
        return pattern_subsumes(ctx, left_pattern_id, *inner_pattern_id);
    }

    match (left_pattern, right_pattern) {
        // exact literal equality
        (
            dir::Pattern::Expression {
                value: left_expression_id,
            },
            dir::Pattern::Expression {
                value: right_expression_id,
            },
        ) => expression_is_equal(ctx, *left_expression_id, *right_expression_id),

        // exact range equality
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
                && optional_pattern_expression_is_equal(ctx, *left_start, *right_start)
                && optional_pattern_expression_is_equal(ctx, *left_end, *right_end)
        }

        // must wrappers are comparable only when wrapper shape matches
        (dir::Pattern::Must(left_inner), dir::Pattern::Must(right_inner)) => {
            pattern_subsumes(ctx, *left_inner, *right_inner)
        }

        // reference wrappers are comparable only with equal mutability
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
            left_mutability == right_mutability && pattern_subsumes(ctx, *left_inner, *right_inner)
        }

        // value wrappers are comparable only with equal mutability
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
            left_mutability == right_mutability && pattern_subsumes(ctx, *left_inner, *right_inner)
        }

        // dereference wrappers are comparable only when wrapper shape matches
        (
            dir::Pattern::DereferenceOf { right: left_inner },
            dir::Pattern::DereferenceOf { right: right_inner },
        ) => pattern_subsumes(ctx, *left_inner, *right_inner),

        _ => false,
    }
}

/// Return true when two optional pattern expressions are equal.
fn optional_pattern_expression_is_equal(
    ctx: &mut LintModuleContext<'_>,
    left: Option<dir::LocalNodeId<dir::Expression>>,
    right: Option<dir::LocalNodeId<dir::Expression>>,
) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => expression_is_equal(ctx, left, right),
        (None, None) => true,
        _ => false,
    }
}
