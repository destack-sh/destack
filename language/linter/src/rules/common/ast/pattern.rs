use destack_ast as ast;

use super::expression_is_equal;
use crate::LintAstContext;

/// Return the expression id when a selector is an expression pattern.
pub fn match_selector_expression_id(
    ctx: &LintAstContext<'_>,
    selector: &ast::MatchSelector,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let pattern_id = selector.pattern_id()?;
    pattern_expression_id(ctx, pattern_id)
}

/// Return one default expression id for a parameter when present.
pub fn parameter_default_expression_id(
    parameter: &ast::Parameter,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    match parameter {
        ast::Parameter::Named { default, .. } | ast::Parameter::Pattern { default, .. } => *default,
        ast::Parameter::VariadicNamed { .. } | ast::Parameter::VariadicPattern { .. } => None,
        ast::Parameter::Error => None,
    }
}

/// Return one default expression id for a pattern field when present.
pub fn pattern_field_default_expression_id(
    tree: &ast::Tree,
    pattern_field: &ast::PatternField,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    match pattern_field {
        ast::PatternField::Named { pattern, .. } => {
            pattern.and_then(|pattern_id| pattern_assignment_value_expression_id(tree, pattern_id))
        }
        ast::PatternField::Computed { pattern, .. } => {
            pattern_assignment_value_expression_id(tree, *pattern)
        }
        ast::PatternField::Positional { pattern } => {
            pattern_assignment_value_expression_id(tree, *pattern)
        }
        ast::PatternField::Spread { .. } | ast::PatternField::Elision => None,
    }
}

/// Return the default value expression for one assignment pattern.
fn pattern_assignment_value_expression_id(
    tree: &ast::Tree,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    match tree.get(pattern_id) {
        ast::Pattern::Assign { value, .. } => Some(*value),
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
        } => pattern_assignment_value_expression_id(tree, *inner_pattern_id),
        ast::Pattern::Wildcard
        | ast::Pattern::Binding { pattern: None, .. }
        | ast::Pattern::Expression { .. }
        | ast::Pattern::TypeExpression { .. }
        | ast::Pattern::Tuple { .. }
        | ast::Pattern::TaggedTuple { .. }
        | ast::Pattern::Array { .. }
        | ast::Pattern::Object { .. }
        | ast::Pattern::TaggedObject { .. }
        | ast::Pattern::Union { .. } => None,
    }
}

/// Return the expression id when a pattern wraps one expression pattern.
pub fn pattern_expression_id(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Expression { value } => Some(*value),
        ast::Pattern::Assign { pattern, .. } => pattern_expression_id(ctx, *pattern),
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
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> bool {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        ast::Pattern::Wildcard => true,
        ast::Pattern::Assign { pattern, .. } => pattern_matches_all(ctx, *pattern),
        ast::Pattern::Binding { pattern, .. } => pattern
            .map(|inner_pattern_id| pattern_matches_all(ctx, inner_pattern_id))
            .unwrap_or(true),
        ast::Pattern::Union { patterns } => patterns
            .iter()
            .any(|pattern_id| pattern_matches_all(ctx, *pattern_id)),
        _ => false,
    }
}

/// Return true when one pattern is `_` or one underscore prefixed binding.
pub fn pattern_is_underscore_binding_or_wildcard(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<ast::Pattern>,
) -> bool {
    // resolve one pattern node
    let pattern = ctx.tree.get(pattern_id);

    // match wildcard and underscore bindings
    match pattern {
        ast::Pattern::Wildcard => true,
        ast::Pattern::Assign { pattern, .. } => {
            pattern_is_underscore_binding_or_wildcard(ctx, *pattern)
        }
        ast::Pattern::Binding { name, .. } => ctx.strings.get(*name).starts_with('_'),
        _ => false,
    }
}

/// Return true when one pattern subsumes another pattern.
pub fn pattern_subsumes(
    ctx: &mut LintAstContext<'_>,
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
