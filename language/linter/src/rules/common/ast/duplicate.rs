use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use destack_ast as ast;
use destack_core::StableHasher;

use crate::LintAstContext;
use crate::rules::common::{
    blocks_equal, expression_is_equal, expression_path_segments, stable_hash_debug,
};

/// Shared duplicate tracker for expression nodes.
#[derive(Debug, Default)]
pub struct ExpressionDuplicateTracker {
    /// Candidate buckets keyed by a coarse expression key.
    buckets: HashMap<u64, Vec<ExpressionDuplicateCandidate>>,
}

#[derive(Debug, Clone, Copy)]
struct ExpressionDuplicateCandidate {
    /// Previously seen expression id.
    expression_id: ast::LocalNodeId<ast::Expression>,
    /// Cached structural key for quick rejection before deep equality.
    structural_key: u64,
}

/// Shared duplicate tracker for block nodes.
#[derive(Debug, Default)]
pub struct BlockDuplicateTracker {
    /// Candidate buckets keyed by a coarse block prefilter.
    buckets: HashMap<u64, Vec<ast::LocalNodeId<ast::Block>>>,
}

impl ExpressionDuplicateTracker {
    /// Build an empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a matching prior expression and insert current when unique.
    pub fn find_duplicate_or_insert(
        &mut self,
        ctx: &LintAstContext<'_>,
        expression_id: ast::LocalNodeId<ast::Expression>,
    ) -> Option<ast::LocalNodeId<ast::Expression>> {
        let key = expression_coarse_key(ctx, expression_id);
        let structural_key = expression_structural_key(ctx, expression_id);
        let candidates = self.buckets.entry(key).or_default();

        for candidate in candidates.iter().copied() {
            if candidate.structural_key != structural_key {
                continue;
            }

            if expression_is_equal(ctx, candidate.expression_id, expression_id) {
                return Some(candidate.expression_id);
            }
        }

        candidates.push(ExpressionDuplicateCandidate {
            expression_id,
            structural_key,
        });
        None
    }
}

impl BlockDuplicateTracker {
    /// Build an empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a matching prior block and insert current when unique.
    pub fn find_duplicate_or_insert(
        &mut self,
        ctx: &LintAstContext<'_>,
        block_id: ast::LocalNodeId<ast::Block>,
    ) -> Option<ast::LocalNodeId<ast::Block>> {
        let key = block_prefilter_key(ctx, block_id);
        let candidates = self.buckets.entry(key).or_default();

        for candidate_id in candidates.iter().copied() {
            if blocks_equal(ctx, candidate_id, block_id) {
                return Some(candidate_id);
            }
        }

        candidates.push(block_id);
        None
    }
}

/// Build a coarse key for expression bucketing.
fn expression_coarse_key(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> u64 {
    let expression = ctx.tree.get(expression_id);
    let expression = unwrap_expression(ctx, expression);

    let mut hasher = StableHasher::new();
    std::mem::discriminant(expression).hash(&mut hasher);

    if let Some(segments) = expression_path_segments(ctx.tree, expression_id) {
        segments.len().hash(&mut hasher);
        return hasher.finish();
    }

    match expression {
        ast::Expression::ScalarLiteral(literal) => {
            std::mem::discriminant(literal).hash(&mut hasher);
        }
        ast::Expression::Type { value } => {
            std::mem::discriminant(ctx.tree.get(*value)).hash(&mut hasher);
        }
        ast::Expression::Binary { operator, .. } => {
            hash_debug_into(&mut hasher, operator);
        }
        ast::Expression::Unary { operator, .. } => {
            hash_debug_into(&mut hasher, operator);
        }
        ast::Expression::Assign { operator, .. } => {
            hash_debug_into(&mut hasher, operator);
        }
        ast::Expression::Call {
            generic_arguments,
            arguments,
            ..
        } => {
            (!generic_arguments.is_empty()).hash(&mut hasher);
            arguments.len().hash(&mut hasher);
        }
        ast::Expression::New {
            generic_arguments,
            arguments,
            ..
        } => {
            (!generic_arguments.is_empty()).hash(&mut hasher);
            arguments.len().hash(&mut hasher);
        }
        ast::Expression::ArrayExpression { elements }
        | ast::Expression::TupleExpression { elements } => {
            elements.len().hash(&mut hasher);
        }
        ast::Expression::ObjectExpression { properties, .. } => {
            properties.len().hash(&mut hasher);
        }
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            block.len().hash(&mut hasher);
        }
        ast::Expression::If {
            kind,
            else_expression,
            ..
        } => {
            hash_debug_into(&mut hasher, kind);
            else_expression.is_some().hash(&mut hasher);
        }
        ast::Expression::Match { kind, cases, .. } => {
            hash_debug_into(&mut hasher, kind);
            cases.len().hash(&mut hasher);
        }
        _ => {}
    }

    hasher.finish()
}

/// Build a structural key used for fast candidate rejection.
fn expression_structural_key(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> u64 {
    let expression = ctx.tree.get(expression_id);
    let expression = unwrap_expression(ctx, expression);

    let mut hasher = StableHasher::new();
    std::mem::discriminant(expression).hash(&mut hasher);

    if let Some(segments) = expression_path_segments(ctx.tree, expression_id) {
        segments.len().hash(&mut hasher);
        for segment in segments {
            segment.hash(&mut hasher);
        }

        return hasher.finish();
    }

    match expression {
        ast::Expression::ScalarLiteral(literal) => {
            hash_debug_into(&mut hasher, literal);
        }
        ast::Expression::Type { value } => {
            hash_debug_into(&mut hasher, ctx.tree.get(*value));
        }
        ast::Expression::Binary {
            operator,
            left,
            right,
        } => {
            hash_debug_into(&mut hasher, operator);
            hash_expression_kind(ctx, &mut hasher, *left);
            hash_expression_kind(ctx, &mut hasher, *right);
        }
        ast::Expression::Unary { operator, right } => {
            hash_debug_into(&mut hasher, operator);
            hash_expression_kind(ctx, &mut hasher, *right);
        }
        ast::Expression::Assign {
            left,
            operator,
            right,
        } => {
            hash_debug_into(&mut hasher, operator);
            hash_assign_pattern_kind(ctx, &mut hasher, *left);
            hash_expression_kind(ctx, &mut hasher, *right);
        }
        ast::Expression::Call {
            left,
            generic_arguments,
            arguments,
            ..
        }
        | ast::Expression::New {
            left,
            generic_arguments,
            arguments,
            ..
        } => {
            hash_expression_kind(ctx, &mut hasher, *left);
            (!generic_arguments.is_empty()).hash(&mut hasher);
            arguments.len().hash(&mut hasher);
            for argument_id in arguments {
                hash_argument_shape(ctx, &mut hasher, *argument_id);
            }
        }
        ast::Expression::ArrayExpression { elements }
        | ast::Expression::TupleExpression { elements } => {
            elements.len().hash(&mut hasher);
            for argument_id in elements {
                hash_argument_shape(ctx, &mut hasher, *argument_id);
            }
        }
        ast::Expression::ObjectExpression { properties, .. } => {
            properties.len().hash(&mut hasher);
        }
        ast::Expression::Block(block_id) => {
            block_prefilter_key(ctx, *block_id).hash(&mut hasher);
        }
        ast::Expression::If {
            kind,
            condition,
            then_expression,
            else_expression,
        } => {
            hash_debug_into(&mut hasher, kind);
            hash_if_condition_shape(ctx, &mut hasher, condition);
            hash_expression_kind(ctx, &mut hasher, *then_expression);
            if let Some(else_expression) = else_expression {
                hash_expression_kind(ctx, &mut hasher, *else_expression);
            } else {
                false.hash(&mut hasher);
            }
        }
        ast::Expression::Match { kind, value, cases } => {
            hash_debug_into(&mut hasher, kind);
            hash_expression_kind(ctx, &mut hasher, *value);
            cases.len().hash(&mut hasher);
        }
        _ => {}
    }

    hasher.finish()
}

/// Hash one assignment pattern shape into the running hasher.
fn hash_assign_pattern_kind(
    ctx: &LintAstContext<'_>,
    hasher: &mut StableHasher,
    assign_pattern_id: ast::LocalNodeId<ast::AssignPattern>,
) {
    let assign_pattern = ctx.tree.get(assign_pattern_id);
    std::mem::discriminant(assign_pattern).hash(hasher);

    match assign_pattern {
        ast::AssignPattern::Expression { value } => {
            hash_expression_kind(ctx, hasher, *value);
        }
        ast::AssignPattern::Assign { pattern, value } => {
            hash_assign_pattern_kind(ctx, hasher, *pattern);
            hash_expression_kind(ctx, hasher, *value);
        }
        ast::AssignPattern::Array { fields } | ast::AssignPattern::Object { fields } => {
            fields.len().hash(hasher);
        }
    }
}

/// Build a coarse prefilter key for one block.
fn block_prefilter_key(ctx: &LintAstContext<'_>, block_id: ast::LocalNodeId<ast::Block>) -> u64 {
    let block = ctx.tree.get(block_id);
    let mut hasher = StableHasher::new();
    block.len().hash(&mut hasher);

    for expression_id in block.iter_expressions() {
        hash_expression_kind(ctx, &mut hasher, expression_id);
    }

    hasher.finish()
}

/// Hash one normalized expression kind.
fn hash_expression_kind(
    ctx: &LintAstContext<'_>,
    hasher: &mut StableHasher,
    expression_id: ast::LocalNodeId<ast::Expression>,
) {
    let expression = ctx.tree.get(expression_id);
    let expression = unwrap_expression(ctx, expression);
    std::mem::discriminant(expression).hash(hasher);
}

/// Hash one argument shape.
fn hash_argument_shape(
    ctx: &LintAstContext<'_>,
    hasher: &mut StableHasher,
    argument_id: ast::LocalNodeId<ast::Argument>,
) {
    let argument = ctx.tree.get(argument_id);
    std::mem::discriminant(argument).hash(hasher);

    match argument {
        ast::Argument::Positional { value, .. } | ast::Argument::Spread { value, .. } => {
            hash_expression_kind(ctx, hasher, *value);
        }
        ast::Argument::Named { name, value, .. } => {
            name.string().hash(hasher);
            hash_expression_kind(ctx, hasher, *value);
        }
        ast::Argument::Labeled { label, value, .. } => {
            label.hash(hasher);
            hash_expression_kind(ctx, hasher, *value);
        }
        ast::Argument::Error => {}
    }
}

/// Hash one if condition shape.
fn hash_if_condition_shape(
    ctx: &LintAstContext<'_>,
    hasher: &mut StableHasher,
    condition: &ast::IfCondition,
) {
    std::mem::discriminant(condition).hash(hasher);

    match condition {
        ast::IfCondition::Expression { condition } => {
            hash_expression_kind(ctx, hasher, *condition);
        }
        ast::IfCondition::Let {
            kind,
            mutability,
            declarator: _,
        } => {
            hash_debug_into(hasher, kind);
            hash_debug_into(hasher, mutability);
        }
    }
}

/// Hash a debug value into the provided hasher.
fn hash_debug_into(hasher: &mut StableHasher, value: &impl std::fmt::Debug) {
    stable_hash_debug(value).hash(hasher);
}

/// Unwrap parenthesized expressions for normalized hashing.
fn unwrap_expression<'a>(
    ctx: &'a LintAstContext<'_>,
    expression: &'a ast::Expression,
) -> &'a ast::Expression {
    match expression {
        ast::Expression::Parenthesized {
            expression: inner_expression_id,
        } => {
            let inner_expression = ctx.tree.get(*inner_expression_id);
            unwrap_expression(ctx, inner_expression)
        }
        _ => expression,
    }
}
