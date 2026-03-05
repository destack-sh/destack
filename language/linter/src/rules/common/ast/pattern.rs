use destack_ast as ast;

use super::expression_is_equal;
use crate::LintModuleAstContext;

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

/// Return the guard expression id for a pattern selector.
pub fn match_selector_guard_expression_id(
    selector: &ast::MatchSelector,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    match selector {
        ast::MatchSelector::Pattern {
            guard: Some(guard_id),
            ..
        } => Some(*guard_id),
        _ => None,
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
