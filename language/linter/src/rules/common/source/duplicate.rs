use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use destack_core::StableHasher;
use destack_dir as dir;

use crate::LintModuleContext;
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
    expression_id: dir::LocalNodeId<dir::Expression>,
    /// Cached structural key for quick rejection before deep equality.
    structural_key: u64,
}

/// Shared duplicate tracker for block nodes.
#[derive(Debug, Default)]
pub struct BlockDuplicateTracker {
    /// Candidate buckets keyed by a coarse block prefilter.
    buckets: HashMap<u64, Vec<dir::LocalNodeId<dir::Block>>>,
}

impl ExpressionDuplicateTracker {
    /// Build an empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a matching prior expression and insert current when unique.
    pub fn find_duplicate_or_insert(
        &mut self,
        ctx: &LintModuleContext<'_>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
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
        ctx: &LintModuleContext<'_>,
        block_id: dir::LocalNodeId<dir::Block>,
    ) -> Option<dir::LocalNodeId<dir::Block>> {
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
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> u64 {
    let expression = ctx.dir.get(expression_id);
    let expression = unwrap_expression(ctx, expression);

    let mut hasher = StableHasher::new();
    std::mem::discriminant(expression).hash(&mut hasher);

    if let Some(segments) = expression_path_segments(ctx.dir.tree(), expression_id) {
        segments.len().hash(&mut hasher);
        return hasher.finish();
    }

    match expression {
        dir::Expression::ScalarLiteral(literal) => {
            std::mem::discriminant(literal).hash(&mut hasher);
        }
        dir::Expression::Type { value } => {
            std::mem::discriminant(ctx.dir.get(*value)).hash(&mut hasher);
        }
        dir::Expression::Binary { operator, .. } => {
            hash_debug_into(&mut hasher, operator);
        }
        dir::Expression::Unary { operator, .. } => {
            hash_debug_into(&mut hasher, operator);
        }
        dir::Expression::Assign { operator, .. } => {
            hash_debug_into(&mut hasher, operator);
        }
        dir::Expression::Call {
            generic_arguments,
            arguments,
            ..
        } => {
            (!generic_arguments.is_empty()).hash(&mut hasher);
            arguments.len().hash(&mut hasher);
        }
        dir::Expression::New {
            generic_arguments,
            arguments,
            ..
        } => {
            (!generic_arguments.is_empty()).hash(&mut hasher);
            arguments.len().hash(&mut hasher);
        }
        dir::Expression::ArrayExpression { elements }
        | dir::Expression::TupleExpression { elements } => {
            elements.len().hash(&mut hasher);
        }
        dir::Expression::ObjectExpression { properties, .. } => {
            properties.len().hash(&mut hasher);
        }
        dir::Expression::Block(block_id) => {
            let block = ctx.dir.get(*block_id);
            block.len().hash(&mut hasher);
        }
        dir::Expression::If {
            form,
            else_expression,
            ..
        } => {
            hash_debug_into(&mut hasher, form);
            else_expression.is_some().hash(&mut hasher);
        }
        dir::Expression::Match { form, cases, .. } => {
            hash_debug_into(&mut hasher, form);
            cases.len().hash(&mut hasher);
        }
        _ => {}
    }

    hasher.finish()
}

/// Build a structural key used for fast candidate rejection.
fn expression_structural_key(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> u64 {
    let expression = ctx.dir.get(expression_id);
    let expression = unwrap_expression(ctx, expression);

    let mut hasher = StableHasher::new();
    std::mem::discriminant(expression).hash(&mut hasher);

    if let Some(segments) = expression_path_segments(ctx.dir.tree(), expression_id) {
        segments.len().hash(&mut hasher);
        for segment in segments {
            segment.hash(&mut hasher);
        }

        return hasher.finish();
    }

    match expression {
        dir::Expression::ScalarLiteral(literal) => {
            hash_debug_into(&mut hasher, literal);
        }
        dir::Expression::Type { value } => {
            hash_debug_into(&mut hasher, ctx.dir.get(*value));
        }
        dir::Expression::Binary {
            operator,
            left,
            right,
        } => {
            hash_debug_into(&mut hasher, operator);
            hash_expression_kind(ctx, &mut hasher, *left);
            hash_expression_kind(ctx, &mut hasher, *right);
        }
        dir::Expression::Unary { operator, right } => {
            hash_debug_into(&mut hasher, operator);
            hash_expression_kind(ctx, &mut hasher, *right);
        }
        dir::Expression::Assign {
            left,
            operator,
            right,
        } => {
            hash_debug_into(&mut hasher, operator);
            hash_assign_pattern_kind(ctx, &mut hasher, *left);
            hash_expression_kind(ctx, &mut hasher, *right);
        }
        dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
            ..
        }
        | dir::Expression::New {
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
        dir::Expression::ArrayExpression { elements }
        | dir::Expression::TupleExpression { elements } => {
            elements.len().hash(&mut hasher);
            for argument_id in elements {
                hash_argument_shape(ctx, &mut hasher, *argument_id);
            }
        }
        dir::Expression::ObjectExpression { properties, .. } => {
            properties.len().hash(&mut hasher);
        }
        dir::Expression::Block(block_id) => {
            block_prefilter_key(ctx, *block_id).hash(&mut hasher);
        }
        dir::Expression::If {
            form,
            condition,
            then_expression,
            else_expression,
        } => {
            hash_debug_into(&mut hasher, form);
            hash_if_condition_shape(ctx, &mut hasher, condition);
            hash_expression_kind(ctx, &mut hasher, *then_expression);
            if let Some(else_expression) = else_expression {
                hash_expression_kind(ctx, &mut hasher, *else_expression);
            } else {
                false.hash(&mut hasher);
            }
        }
        dir::Expression::Match { form, value, cases } => {
            hash_debug_into(&mut hasher, form);
            hash_expression_kind(ctx, &mut hasher, *value);
            cases.len().hash(&mut hasher);
        }
        _ => {}
    }

    hasher.finish()
}

/// Hash one assignment pattern shape into the running hasher.
fn hash_assign_pattern_kind(
    ctx: &LintModuleContext<'_>,
    hasher: &mut StableHasher,
    assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
) {
    let assign_pattern = ctx.dir.get(assign_pattern_id);
    std::mem::discriminant(assign_pattern).hash(hasher);

    match assign_pattern {
        dir::AssignPattern::Expression { value } => {
            hash_expression_kind(ctx, hasher, *value);
        }
        dir::AssignPattern::Assign { pattern, value } => {
            hash_assign_pattern_kind(ctx, hasher, *pattern);
            hash_expression_kind(ctx, hasher, *value);
        }
        dir::AssignPattern::Sequence { fields } | dir::AssignPattern::Object { fields } => {
            fields.len().hash(hasher);
        }
    }
}

/// Build a coarse prefilter key for one block.
fn block_prefilter_key(ctx: &LintModuleContext<'_>, block_id: dir::LocalNodeId<dir::Block>) -> u64 {
    let block = ctx.dir.get(block_id);
    let mut hasher = StableHasher::new();
    block.len().hash(&mut hasher);

    for expression_id in block.iter_expressions() {
        hash_expression_kind(ctx, &mut hasher, expression_id);
    }

    hasher.finish()
}

/// Hash one normalized expression kind.
fn hash_expression_kind(
    ctx: &LintModuleContext<'_>,
    hasher: &mut StableHasher,
    expression_id: dir::LocalNodeId<dir::Expression>,
) {
    let expression = ctx.dir.get(expression_id);
    let expression = unwrap_expression(ctx, expression);
    std::mem::discriminant(expression).hash(hasher);
}

/// Hash one argument shape.
fn hash_argument_shape(
    ctx: &LintModuleContext<'_>,
    hasher: &mut StableHasher,
    argument_id: dir::LocalNodeId<dir::Argument>,
) {
    let argument = ctx.dir.get(argument_id);
    std::mem::discriminant(argument).hash(hasher);

    match argument {
        dir::Argument::Positional { value, .. } | dir::Argument::Spread { value, .. } => {
            hash_expression_kind(ctx, hasher, *value);
        }
        dir::Argument::Named { name, value, .. } => {
            name.string().hash(hasher);
            hash_expression_kind(ctx, hasher, *value);
        }
        dir::Argument::Labeled { label, value, .. } => {
            label.hash(hasher);
            hash_expression_kind(ctx, hasher, *value);
        }
        dir::Argument::Error => {}
    }
}

/// Hash one if condition shape.
fn hash_if_condition_shape(
    ctx: &LintModuleContext<'_>,
    hasher: &mut StableHasher,
    condition: &dir::IfCondition,
) {
    std::mem::discriminant(condition).hash(hasher);

    match condition {
        dir::IfCondition::Expression { condition } => {
            hash_expression_kind(ctx, hasher, *condition);
        }
        dir::IfCondition::Let {
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
    ctx: &'a LintModuleContext<'_>,
    expression: &'a dir::Expression,
) -> &'a dir::Expression {
    match expression {
        dir::Expression::Parenthesized {
            expression: inner_expression_id,
        } => {
            let inner_expression = ctx.dir.get(*inner_expression_id);
            unwrap_expression(ctx, inner_expression)
        }
        _ => expression,
    }
}
