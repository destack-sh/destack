use ast::{
    AnnotationPosition, Declaration, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenType,
};
use destack_ast as ast;

use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner,
};
use super::facts::{previous_non_trivia_token_index, token_type_is_trivia};
use super::ownership::{
    find_owner_in_ancestor_chain, find_owner_in_candidate_ancestry,
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    promote_owner_by_shared_start, promote_owner_to_node_type_ancestor,
    promote_owner_to_satisfies_expression_ancestor, promote_rhs_expression_owner,
};

/// Promote one owner to the nearest elementwise binary expression ancestor.
fn promote_owner_to_elementwise_binary_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if matches!(
                tree.get(expression_id),
                Expression::Binary {
                    operator: ast::BinaryOperator::ElementwiseAnd
                        | ast::BinaryOperator::ElementwiseOr
                        | ast::BinaryOperator::ElementwiseXor,
                    ..
                }
            ) {
                return Some(node_id);
            }
        }

        None
    })
}

/// Descend one owner to the terminal right operand of one elementwise binary chain.
fn descend_owner_to_terminal_elementwise_operand(tree: &NodeTree, owner_id: u32) -> u32 {
    let mut current_id = owner_id;

    loop {
        if tree.get_node_type(current_id) != NodeType::Expression {
            return current_id;
        }

        let expression_id = LocalNodeId::<Expression>::new(current_id);
        let Expression::Binary {
            operator:
                ast::BinaryOperator::ElementwiseAnd
                | ast::BinaryOperator::ElementwiseOr
                | ast::BinaryOperator::ElementwiseXor,
            right,
            ..
        } = tree.get(expression_id)
        else {
            return current_id;
        };

        current_id = right.id;
    }
}

/// Return whether two owners are in the same elementwise binary chain.
fn owners_share_elementwise_binary_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    left_owner_id: u32,
    right_owner_id: u32,
) -> bool {
    let left_binary_owner =
        promote_owner_to_elementwise_binary_expression_ancestor(tree, parents, left_owner_id);
    let right_binary_owner =
        promote_owner_to_elementwise_binary_expression_ancestor(tree, parents, right_owner_id);

    left_binary_owner.is_some() && left_binary_owner == right_binary_owner
}

/// Return whether one owner is one cast-like type-binary expression.
fn is_cast_or_satisfies_type_binary_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::TypeBinary {
            operator: ast::TypeBinaryOperator::Cast | ast::TypeBinaryOperator::Satisfies,
            ..
        }
    )
}

/// Promote one owner to the nearest cast or satisfies type-binary expression ancestor.
fn promote_owner_to_cast_or_satisfies_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        is_cast_or_satisfies_type_binary_expression_owner(tree, node_id).then_some(node_id)
    })
}

/// Return whether one rhs owner is one single-segment path with multiple static arguments.
fn rhs_is_single_segment_multi_static_argument_path(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::Path {
            path,
            static_arguments: Some(static_arguments),
        } if path.segments.len() == 1 && static_arguments.len() > 1
    )
}

/// Promote one owner to the mapped-type key-remap path owner.
fn promote_owner_to_mapped_type_key_remap_path_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if let Expression::TypeTemplateLiteral { spans, .. } = tree.get(expression_id)
                && let Some(parent_id) = parents.get_by_id(node_id)
                && tree.get_node_type(parent_id) == NodeType::Expression
            {
                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                if let Expression::TypeMapped { parameter, .. } = tree.get(parent_expression_id)
                    && parameter.key_remap == Some(LocalNodeId::new(node_id))
                    && let Some(first_span) = spans.first().copied()
                    && matches!(
                        tree.get(first_span),
                        Expression::Path {
                            static_arguments: Some(_),
                            ..
                        }
                    )
                {
                    return Some(first_span.id);
                }
            }
        }

        None
    })
}

/// Promote one owner to the nearest mapped-type value owner.
fn promote_owner_to_mapped_type_value_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if let Expression::TypeMapped { value, .. } = tree.get(expression_id) {
                return Some(value.id);
            }
        }

        None
    })
}

/// Return assignment token index for one seam when comments follow `=`.
fn assignment_token_index_for_seam(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> Option<usize> {
    if seam.token_before_is(TokenType::Assign) {
        return context.token_before;
    }

    let token_before_index = context.token_before?;
    let token_before_type = context.semantic_tokens[token_before_index].token.ty;
    if !token_type_is_trivia(token_before_type) {
        return None;
    }

    let previous_index =
        previous_non_trivia_token_index(context.semantic_tokens, token_before_index)?;
    let previous_type = context.semantic_tokens[previous_index].token.ty;
    (previous_type == TokenType::Assign).then_some(previous_index)
}

/// Return rhs owner for one assignment seam.
fn assignment_owner_from_token_index(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    token_index: usize,
) -> Option<u32> {
    let token_span = context.semantic_tokens[token_index].span;
    let owner_id = find_smallest_owner_enclosing_token(tree, token_span)?;

    let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
        Some(owner_id)
    } else {
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
    };
    if let Some(expression_owner) = expression_owner {
        let expression_id = LocalNodeId::<Expression>::new(expression_owner);
        if let Expression::Assign { right, .. } = tree.get(expression_id) {
            return Some(right.id);
        }
    }

    let declaration_owner = if tree.get_node_type(owner_id) == NodeType::Declaration {
        Some(owner_id)
    } else {
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Declaration)
    }?;
    let declaration_id = LocalNodeId::<Declaration>::new(declaration_owner);
    match tree.get(declaration_id) {
        Declaration::Type { value, .. } => Some(value.id),
        _ => None,
    }
}

/// Return rhs owner for one assignment seam fallback chain.
fn assignment_rhs_owner_fallback(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    following_owner: Option<u32>,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
) -> Option<u32> {
    context
        .token_after_span
        .and_then(|token_after| find_smallest_owner_enclosing_token(tree, token_after.span))
        .or(following_owner)
        .or_else(|| comment_enclosing_owner(context, enclosing_owner_cache))
}

/// Return rhs owner for one assignment seam.
fn assignment_rhs_owner_for_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
) -> Option<u32> {
    let assignment_owner = assignment_token_index_for_seam(context, seam).and_then(|token_index| {
        assignment_owner_from_token_index(tree, parents, context, token_index)
    });
    let target_node = assignment_owner.or_else(|| {
        assignment_rhs_owner_fallback(tree, context, following_owner, enclosing_owner_cache)
    })?;

    let mut target_node = promote_rhs_expression_owner(
        tree,
        parents,
        target_node,
        context.token_after_span.map(|token| token.span),
    );

    if tree.get_node_type(target_node) == NodeType::Declaration {
        let declaration_id = LocalNodeId::<Declaration>::new(target_node);
        if let Declaration::Type { value, .. } = tree.get(declaration_id) {
            target_node = value.id;
        }
    }

    Some(target_node)
}

/// Resolve assignment seam comment rules.
fn assignment_seam_attachment_position(
    seam: &CommentSeamData,
    comment_starts_on_assignment_line: bool,
) -> Option<AnnotationPosition> {
    // inline block comments between assignment and rhs should stay inline with the rhs expression
    if !seam.has_leading_newline
        && !seam.has_trailing_newline
        && seam.comment_is_star
        && !seam.comment_is_line
    {
        return Some(AnnotationPosition::LinePrefix);
    }

    // comments between assignment and rhs should bind to the rhs seam
    if !seam.has_leading_newline && seam.has_trailing_newline {
        if seam.comment_is_line && comment_starts_on_assignment_line {
            return Some(AnnotationPosition::LinePrefix);
        }

        return Some(AnnotationPosition::BlockPrefix);
    }

    // own-line comments between assignment and rhs stay on the rhs value region
    if seam.has_leading_newline {
        return Some(AnnotationPosition::BlockPrefix);
    }

    None
}

/// Resolve assignment seam comment rules.
pub(crate) fn try_attach_comment_assignment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let following_owner = owners.following;
    let assignment_token_index = assignment_token_index_for_seam(context, seam);
    let seam_follows_assignment = assignment_token_index.is_some();
    if !seam_follows_assignment {
        return None;
    }

    let comment_starts_on_assignment_line = assignment_token_index
        .and_then(|index| context.semantic_tokens.get(index).copied())
        .is_some_and(|token| {
            context
                .file
                .is_same_line(token.span.start, context.trivia.span.start)
        });

    let target_node = assignment_rhs_owner_for_seam(
        tree,
        parents,
        context,
        seam,
        following_owner,
        enclosing_owner_cache,
    )?;
    let position = assignment_seam_attachment_position(seam, comment_starts_on_assignment_line)?;

    Some((Some(target_node), position))
}

/// Return whether token after one seam is one elementwise operator.
fn seam_token_after_is_elementwise_operator(seam: &CommentSeamData) -> bool {
    matches!(
        seam.token_after_type,
        Some(TokenType::ElementwiseAnd | TokenType::ElementwiseOr | TokenType::ElementwiseXor)
    )
}

/// Return whether one seam starts one leading type-grouping operator region.
fn seam_starts_leading_type_grouping_operator(
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
) -> bool {
    seam.token_before_is(TokenType::OpenParenthesis)
        || seam.token_before_is(TokenType::Assign)
        || seam.token_before_is(TokenType::Colon)
        || seam.token_before_is_keyword(CommentSeamKeyword::As)
        || seam.token_before_is_keyword(CommentSeamKeyword::Satisfies)
        || preceding_owner.is_none()
}

/// Resolve one mapped-type value owner near one seam.
fn mapped_type_value_owner_for_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<u32> {
    [following_owner, preceding_owner, enclosing_owner]
        .into_iter()
        .flatten()
        .find_map(|owner_id| promote_owner_to_mapped_type_value_owner(tree, parents, owner_id))
}

/// Resolve assignment seams that lead into type-grouping operators.
fn try_attach_assignment_leading_grouping_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam_token_after_is_elementwise_operator(seam)
        || !seam.token_before_is(TokenType::Assign)
        || !(seam.comment_is_star || seam.comment_is_line)
    {
        return None;
    }

    let target_owner = assignment_rhs_owner_for_seam(
        tree,
        parents,
        context,
        seam,
        following_owner,
        enclosing_owner_cache,
    )
    .or(following_owner)
    .or(enclosing_owner)?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    let position = if seam.has_trailing_newline || seam.comment_is_multiline_star {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };
    Some((Some(target_owner), position))
}

/// Resolve leading type-grouping operator comments.
fn try_attach_leading_type_grouping_operator_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam_token_after_is_elementwise_operator(seam)
        || !seam_starts_leading_type_grouping_operator(seam, preceding_owner)
        || !(seam.comment_is_star || seam.comment_is_line)
    {
        return None;
    }

    let target_owner = following_owner
        .or_else(|| {
            context
                .token_after_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        })
        .or(enclosing_owner)?;
    let target_owner =
        promote_owner_to_elementwise_binary_expression_ancestor(tree, parents, target_owner)
            .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    let position = if seam.has_trailing_newline || seam.comment_is_multiline_star {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };
    Some((Some(target_owner), position))
}

/// Resolve own-line comments after mapped-type value `:`.
fn try_attach_mapped_type_value_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.token_before_is(TokenType::Colon)
        || seam.token_before_is_return_type_colon
        || !seam.has_leading_newline
        || !seam.comment_is_line
    {
        return None;
    }

    let target_owner = mapped_type_value_owner_for_seam(
        tree,
        parents,
        preceding_owner,
        following_owner,
        enclosing_owner,
    )?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Resolve line comments before type and bitwise separators.
fn try_attach_comment_before_elementwise_separator(
    tree: &NodeTree,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.has_trailing_newline
        || !seam.comment_is_line
        || !seam_token_after_is_elementwise_operator(seam)
    {
        return None;
    }

    let target_owner = preceding_owner?;
    let target_owner = descend_owner_to_terminal_elementwise_operand(tree, target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    let position = if seam.has_leading_newline {
        AnnotationPosition::LinePostfixBoundary
    } else {
        AnnotationPosition::LinePostfix
    };
    Some((Some(target_owner), position))
}

/// Resolve trailing line comments after mapped-type value `:`.
fn try_attach_mapped_type_value_trailing_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.token_before_is(TokenType::Colon)
        || seam.token_before_is_return_type_colon
        || seam.has_leading_newline
        || !seam.has_trailing_newline
        || !seam.comment_is_line
    {
        return None;
    }

    let target_owner = mapped_type_value_owner_for_seam(
        tree,
        parents,
        preceding_owner,
        following_owner,
        enclosing_owner,
    )?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve assignment and leading grouping operator seam comment rules.
struct AssignmentAndGroupingDispatchContext<'a, 'cache> {
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// The seam context.
    seam_ctx: &'a CommentSeamContext<'a>,
    /// The seam facts.
    seam: &'a CommentSeamData,
    /// Mutable enclosing owner cache.
    enclosing_owner_cache: &'cache mut CommentEnclosingOwnerCache,
    /// Neighbor owner candidates.
    following_owner: Option<u32>,
    /// Neighbor owner candidates.
    preceding_owner: Option<u32>,
    /// Enclosing owner candidate.
    enclosing_owner: Option<u32>,
}

/// One assignment/grouping seam handler in priority order.
type AssignmentAndGroupingHandler =
    fn(&mut AssignmentAndGroupingDispatchContext<'_, '_>) -> Option<CommentAttachment>;

/// Run ordered assignment/grouping seam handlers.
fn run_assignment_and_grouping_handlers(
    ctx: &mut AssignmentAndGroupingDispatchContext<'_, '_>,
    handlers: &[AssignmentAndGroupingHandler],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Attach assignment-leading grouping comments.
fn attach_assignment_and_grouping_assignment_leading(
    ctx: &mut AssignmentAndGroupingDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_assignment_leading_grouping_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.following_owner,
        ctx.enclosing_owner,
    )
}

/// Attach leading type-grouping operator comments.
fn attach_assignment_and_grouping_leading_type_grouping(
    ctx: &mut AssignmentAndGroupingDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_leading_type_grouping_operator_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.enclosing_owner,
    )
}

/// Attach mapped-type own-line comments after `:`.
fn attach_assignment_and_grouping_mapped_type_own_line(
    ctx: &mut AssignmentAndGroupingDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_mapped_type_value_own_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.enclosing_owner,
    )
}

/// Attach line comments before elementwise separators.
fn attach_assignment_and_grouping_before_elementwise_separator(
    ctx: &mut AssignmentAndGroupingDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_before_elementwise_separator(ctx.tree, ctx.seam, ctx.preceding_owner)
}

/// Attach mapped-type trailing line comments after `:`.
fn attach_assignment_and_grouping_mapped_type_trailing_line(
    ctx: &mut AssignmentAndGroupingDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_mapped_type_value_trailing_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.enclosing_owner,
    )
}

/// Ordered assignment/grouping seam handlers.
const ASSIGNMENT_AND_GROUPING_HANDLERS: &[AssignmentAndGroupingHandler] = &[
    attach_assignment_and_grouping_assignment_leading,
    attach_assignment_and_grouping_leading_type_grouping,
    attach_assignment_and_grouping_mapped_type_own_line,
    attach_assignment_and_grouping_before_elementwise_separator,
    attach_assignment_and_grouping_mapped_type_trailing_line,
];

/// Resolve assignment and leading grouping operator seam comment rules.
fn try_attach_comment_expression_operator_assignment_and_leading_grouping(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    following_owner: Option<u32>,
    preceding_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let mut ctx = AssignmentAndGroupingDispatchContext {
        tree,
        parents,
        seam_ctx: context,
        seam,
        enclosing_owner_cache,
        following_owner,
        preceding_owner,
        enclosing_owner,
    };

    run_assignment_and_grouping_handlers(&mut ctx, ASSIGNMENT_AND_GROUPING_HANDLERS)
}

/// Resolve cast and satisfies seam comment rules.
fn seam_candidate_owner_ids(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> [Option<u32>; 5] {
    let token_before_owner = context
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let token_after_owner = context
        .token_after_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));

    [
        enclosing_owner,
        preceding_owner,
        following_owner,
        token_before_owner,
        token_after_owner,
    ]
}

/// Resolve cast and satisfies seam comment rules.
fn cast_or_satisfies_expression_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<u32> {
    let seam_candidate_owners = seam_candidate_owner_ids(
        tree,
        context,
        preceding_owner,
        following_owner,
        enclosing_owner,
    );

    find_owner_in_candidate_ancestry(parents, seam_candidate_owners, |owner_id| {
        promote_owner_to_cast_or_satisfies_expression_ancestor(tree, parents, owner_id)
    })
}

/// Resolve one rhs owner from a cast or satisfies expression owner.
fn cast_or_satisfies_expression_rhs_owner(
    tree: &NodeTree,
    expression_owner: Option<u32>,
) -> Option<u32> {
    let expression_owner = expression_owner?;
    let expression_id = LocalNodeId::<Expression>::new(expression_owner);
    match tree.get(expression_id) {
        Expression::TypeBinary { right, .. } => Some(right.id),
        _ => None,
    }
}

/// Resolve one `as const` unary expression owner near one seam.
fn as_const_type_unary_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<u32> {
    let seam_candidate_owners = seam_candidate_owner_ids(
        tree,
        context,
        preceding_owner,
        following_owner,
        enclosing_owner,
    );

    find_owner_in_candidate_ancestry(parents, seam_candidate_owners, |owner_id| {
        let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
            Some(owner_id)
        } else {
            promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
        }?;

        let expression_id = LocalNodeId::<Expression>::new(expression_owner);
        match tree.get(expression_id) {
            Expression::TypeUnary {
                operator: ast::TypeUnaryOperator::AsConst,
                ..
            } => Some(expression_owner),
            _ => None,
        }
    })
}

/// Promote one seam target by token-after shared start when available.
fn promote_seam_target_by_token_after_start(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    target_owner: u32,
) -> u32 {
    context
        .token_after_span
        .map(|token| promote_owner_by_shared_start(tree, parents, target_owner, token.span.start))
        .unwrap_or(target_owner)
}

/// Resolve star comments before `as` or `satisfies` on one cast seam.
fn try_attach_cast_seam_inline_star_before_operator(
    tree: &NodeTree,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline || seam.has_trailing_newline || !seam.comment_is_star {
        return None;
    }

    if !(seam.token_after_is_keyword(CommentSeamKeyword::As)
        || seam.token_after_is_keyword(CommentSeamKeyword::Satisfies))
    {
        return None;
    }

    let target_owner = preceding_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfix))
}

/// Resolve multiline star comments after cast operators.
fn try_attach_cast_seam_multiline_star_after_operator(
    tree: &NodeTree,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    cast_expression_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !(seam.token_before_is_keyword(CommentSeamKeyword::As)
        || seam.token_before_is_keyword(CommentSeamKeyword::Satisfies))
    {
        return None;
    }

    if !seam.comment_is_multiline_star || (!seam.has_leading_newline && !seam.has_trailing_newline)
    {
        return None;
    }

    cast_expression_owner?;

    let target_owner = following_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Resolve own-line cast and satisfies seam comments.
fn try_attach_cast_seam_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    cast_expression_owner: Option<u32>,
    cast_rhs_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline || !seam.has_trailing_newline || !seam.comment_is_line {
        return None;
    }

    cast_expression_owner?;

    let target_owner = cast_rhs_owner.or(following_owner).or_else(|| {
        context
            .token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
    })?;
    let target_owner =
        promote_seam_target_by_token_after_start(tree, parents, context, target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Resolve mapped-type key-remap line comments after `as`.
fn try_attach_cast_mapped_type_key_remap_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.token_before_is_keyword(CommentSeamKeyword::As)
        || seam.has_leading_newline
        || !seam.has_trailing_newline
        || !seam.comment_is_line
    {
        return None;
    }

    let target_owner = [following_owner, enclosing_owner, preceding_owner]
        .into_iter()
        .flatten()
        .find_map(|owner_id| {
            promote_owner_to_mapped_type_key_remap_path_owner(tree, parents, owner_id)
        })?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfix))
}

/// Resolve line comments after `as` or `satisfies`.
fn cast_seam_is_trailing_line_after_operator(seam: &CommentSeamData) -> bool {
    if !(seam.token_before_is_keyword(CommentSeamKeyword::As)
        || seam.token_before_is_keyword(CommentSeamKeyword::Satisfies))
    {
        return false;
    }

    !seam.has_leading_newline && seam.has_trailing_newline && seam.comment_is_line
}

/// Return whether one satisfies seam keeps rhs-prefix placement.
fn cast_seam_keeps_rhs_prefix(
    tree: &NodeTree,
    seam: &CommentSeamData,
    cast_rhs_owner: Option<u32>,
) -> bool {
    seam.token_before_is_keyword(CommentSeamKeyword::Satisfies)
        && cast_rhs_owner.is_some_and(|owner_id| {
            rhs_is_single_segment_multi_static_argument_path(tree, owner_id)
        })
}

/// Resolve rhs-prefix target owner for cast seam line comments.
fn cast_seam_rhs_prefix_target_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    following_owner: Option<u32>,
    cast_rhs_owner: Option<u32>,
) -> Option<u32> {
    let target_owner = cast_rhs_owner.or(following_owner).or_else(|| {
        context
            .token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
    })?;
    let target_owner =
        promote_seam_target_by_token_after_start(tree, parents, context, target_owner);

    Some(normalize_formatter_trivia_target_owner(tree, target_owner))
}

/// Resolve one `as const` trailing line comment.
fn try_attach_as_const_cast_seam_trailing_line_comment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    as_const_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.token_before_is_keyword(CommentSeamKeyword::As)
        || !seam.token_after_is_keyword(CommentSeamKeyword::Const)
    {
        return None;
    }

    let target_owner = as_const_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfix))
}

/// Resolve one trailing cast-expression boundary line comment.
fn try_attach_cast_expression_trailing_line_comment(
    tree: &NodeTree,
    cast_expression_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let target_owner = cast_expression_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);

    Some((Some(target_owner), AnnotationPosition::LinePostfix))
}

/// Resolve line comments after `as` or `satisfies`.
fn try_attach_cast_seam_trailing_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
    cast_expression_owner: Option<u32>,
    cast_rhs_owner: Option<u32>,
    as_const_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !cast_seam_is_trailing_line_after_operator(seam) {
        return None;
    }

    // satisfies keeps rhs prefix only for single-segment static-argument paths
    if cast_seam_keeps_rhs_prefix(tree, seam, cast_rhs_owner)
        && let Some(target_owner) = cast_seam_rhs_prefix_target_owner(
            tree,
            parents,
            context,
            following_owner,
            cast_rhs_owner,
        )
    {
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    // `as const` line comments stay on the unary `as const` owner
    if let Some(attachment) =
        try_attach_as_const_cast_seam_trailing_line_comment(tree, seam, as_const_owner)
    {
        return Some(attachment);
    }

    // default cast and satisfies line comments stay on the expression boundary
    try_attach_cast_expression_trailing_line_comment(tree, cast_expression_owner)
}

/// Resolve line comments after `<` in satisfies rhs type arguments.
fn try_attach_satisfies_rhs_type_argument_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.token_before_is(TokenType::LessThan)
        || seam.has_leading_newline
        || !seam.has_trailing_newline
        || !seam.comment_is_line
    {
        return None;
    }

    let has_satisfies_ancestor = comment_enclosing_owner(context, enclosing_owner_cache)
        .or(preceding_owner)
        .and_then(|target_owner| {
            promote_owner_to_satisfies_expression_ancestor(tree, parents, target_owner)
        })
        .is_some();
    if !has_satisfies_ancestor {
        return None;
    }

    let target_owner = following_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Resolve cast and satisfies seam comment rules.
struct CastAndSatisfiesDispatchContext<'a, 'cache> {
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// The seam context.
    seam_ctx: &'a CommentSeamContext<'a>,
    /// The seam facts.
    seam: &'a CommentSeamData,
    /// Mutable enclosing owner cache.
    enclosing_owner_cache: &'cache mut CommentEnclosingOwnerCache,
    /// Neighbor owner candidates.
    preceding_owner: Option<u32>,
    /// Neighbor owner candidates.
    following_owner: Option<u32>,
    /// Enclosing owner candidate.
    enclosing_owner: Option<u32>,
    /// Cast or satisfies expression owner.
    cast_expression_owner: Option<u32>,
    /// Rhs owner inside cast or satisfies expression.
    cast_rhs_owner: Option<u32>,
    /// `as const` unary owner.
    as_const_owner: Option<u32>,
}

/// One cast/satisfies seam handler in priority order.
type CastAndSatisfiesHandler =
    fn(&mut CastAndSatisfiesDispatchContext<'_, '_>) -> Option<CommentAttachment>;

/// Run ordered cast/satisfies seam handlers.
fn run_cast_and_satisfies_handlers(
    ctx: &mut CastAndSatisfiesDispatchContext<'_, '_>,
    handlers: &[CastAndSatisfiesHandler],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Attach cast seam inline star comments before operator.
fn attach_cast_and_satisfies_inline_star_before_operator(
    ctx: &mut CastAndSatisfiesDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_cast_seam_inline_star_before_operator(ctx.tree, ctx.seam, ctx.preceding_owner)
}

/// Attach cast seam multiline star comments after operator.
fn attach_cast_and_satisfies_multiline_star_after_operator(
    ctx: &mut CastAndSatisfiesDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_cast_seam_multiline_star_after_operator(
        ctx.tree,
        ctx.seam,
        ctx.following_owner,
        ctx.cast_expression_owner,
    )
}

/// Attach own-line cast and satisfies seam comments.
fn attach_cast_and_satisfies_own_line(
    ctx: &mut CastAndSatisfiesDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_cast_seam_own_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.following_owner,
        ctx.cast_expression_owner,
        ctx.cast_rhs_owner,
    )
}

/// Attach mapped-type key-remap comments after `as`.
fn attach_cast_and_satisfies_mapped_type_key_remap(
    ctx: &mut CastAndSatisfiesDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_cast_mapped_type_key_remap_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.enclosing_owner,
    )
}

/// Attach trailing line comments after `as` and `satisfies`.
fn attach_cast_and_satisfies_trailing_line(
    ctx: &mut CastAndSatisfiesDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_cast_seam_trailing_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.following_owner,
        ctx.cast_expression_owner,
        ctx.cast_rhs_owner,
        ctx.as_const_owner,
    )
}

/// Attach satisfies rhs type argument comments after `<`.
fn attach_cast_and_satisfies_rhs_type_argument(
    ctx: &mut CastAndSatisfiesDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_satisfies_rhs_type_argument_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.preceding_owner,
        ctx.following_owner,
    )
}

/// Ordered cast and satisfies seam handlers.
const CAST_AND_SATISFIES_HANDLERS: &[CastAndSatisfiesHandler] = &[
    attach_cast_and_satisfies_inline_star_before_operator,
    attach_cast_and_satisfies_multiline_star_after_operator,
    attach_cast_and_satisfies_own_line,
    attach_cast_and_satisfies_mapped_type_key_remap,
    attach_cast_and_satisfies_trailing_line,
    attach_cast_and_satisfies_rhs_type_argument,
];

/// Resolve cast and satisfies seam comment rules.
fn try_attach_comment_expression_operator_cast_and_satisfies(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let cast_expression_owner = cast_or_satisfies_expression_owner(
        tree,
        parents,
        context,
        preceding_owner,
        following_owner,
        enclosing_owner,
    );
    let cast_rhs_owner = cast_or_satisfies_expression_rhs_owner(tree, cast_expression_owner);
    let as_const_owner = as_const_type_unary_owner(
        tree,
        parents,
        context,
        preceding_owner,
        following_owner,
        enclosing_owner,
    );
    let mut ctx = CastAndSatisfiesDispatchContext {
        tree,
        parents,
        seam_ctx: context,
        seam,
        enclosing_owner_cache,
        preceding_owner,
        following_owner,
        enclosing_owner,
        cast_expression_owner,
        cast_rhs_owner,
        as_const_owner,
    };

    run_cast_and_satisfies_handlers(&mut ctx, CAST_AND_SATISFIES_HANDLERS)
}

/// Resolve elementwise operator seam comment rules.
fn seam_token_before_is_elementwise_operator(seam: &CommentSeamData) -> bool {
    matches!(
        seam.token_before_type,
        Some(TokenType::ElementwiseAnd | TokenType::ElementwiseOr | TokenType::ElementwiseXor)
    )
}

/// Resolve inline star comments after elementwise operators.
fn try_attach_elementwise_inline_star_rhs_comment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam_token_before_is_elementwise_operator(seam)
        || seam.has_trailing_newline
        || !seam.comment_is_star
        || preceding_owner.is_some()
    {
        return None;
    }

    let target_owner = following_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Resolve star comments with trailing newline after elementwise operators.
fn try_attach_elementwise_star_rhs_comment(
    tree: &NodeTree,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam_token_before_is_elementwise_operator(seam)
        || !seam.has_trailing_newline
        || !seam.comment_is_star
    {
        return None;
    }

    let target_owner = following_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    let position = if seam.has_leading_newline || seam.comment_is_multiline_star {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };
    Some((Some(target_owner), position))
}

/// Resolve line comments after elementwise operators.
fn try_attach_elementwise_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam_token_before_is_elementwise_operator(seam)
        || seam.has_leading_newline
        || !seam.has_trailing_newline
        || !seam.comment_is_line
    {
        return None;
    }

    // line comments after `&` should stay with the rhs type operand
    if seam.token_before_is(TokenType::ElementwiseAnd)
        && let Some(target_owner) = context
            .token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(following_owner)
    {
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    // line comments after `|` and `^` within one chain stay on the terminal left operand
    if (seam.token_before_is(TokenType::ElementwiseOr)
        || seam.token_before_is(TokenType::ElementwiseXor))
        && let Some(preceding_owner) = preceding_owner
        && let Some(following_owner) = following_owner
        && owners_share_elementwise_binary_expression_ancestor(
            tree,
            parents,
            preceding_owner,
            following_owner,
        )
    {
        let target_owner = descend_owner_to_terminal_elementwise_operand(tree, preceding_owner);
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    // default: keep line comment with the rhs operand
    let target_owner = following_owner?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Resolve elementwise operator seam comment rules.
fn try_attach_comment_expression_operator_elementwise(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    // line comments after type and bitwise operators should stay with the rhs operand
    if let Some(attachment) =
        try_attach_elementwise_inline_star_rhs_comment(tree, seam, preceding_owner, following_owner)
    {
        return Some(attachment);
    }

    // multiline or own-line block comments after type and bitwise operators stay with the rhs operand
    if let Some(attachment) = try_attach_elementwise_star_rhs_comment(tree, seam, following_owner) {
        return Some(attachment);
    }

    // line comments after type and bitwise operators should stay on the left operand boundary
    if let Some(attachment) = try_attach_elementwise_line_comment(
        tree,
        parents,
        context,
        seam,
        preceding_owner,
        following_owner,
    ) {
        return Some(attachment);
    }

    None
}

/// Resolve one multiline `as const` seam comment.
fn try_attach_comment_expression_operator_as_const_multiline(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
) -> Option<CommentAttachment> {
    // multiline comments between `as` and `const` stay after the assertion
    if seam.token_before_is_keyword(CommentSeamKeyword::As)
        && seam.token_after_is_keyword(CommentSeamKeyword::Const)
        && !seam.has_leading_newline
        && seam.comment_is_multiline_star
        && let Some(target_node) = comment_enclosing_owner(context, enclosing_owner_cache)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Resolve expression operator seam comment rules.
struct ExpressionOperatorDispatchContext<'a, 'cache> {
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// The seam context.
    seam_ctx: &'a CommentSeamContext<'a>,
    /// The seam facts.
    seam: &'a CommentSeamData,
    /// Mutable enclosing owner cache.
    enclosing_owner_cache: &'cache mut CommentEnclosingOwnerCache,
    /// Neighbor owner candidates.
    owners: CommentAttachmentNeighbors,
    /// Enclosing owner candidate.
    enclosing_owner: Option<u32>,
}

/// One expression-operator seam handler in priority order.
type ExpressionOperatorHandler =
    fn(&mut ExpressionOperatorDispatchContext<'_, '_>) -> Option<CommentAttachment>;

/// Run ordered expression-operator seam handlers.
fn run_expression_operator_handlers(
    ctx: &mut ExpressionOperatorDispatchContext<'_, '_>,
    handlers: &[ExpressionOperatorHandler],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Attach assignment and leading-grouping expression operator seams.
fn attach_expression_operator_assignment_and_grouping(
    ctx: &mut ExpressionOperatorDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_expression_operator_assignment_and_leading_grouping(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.owners.following,
        ctx.owners.preceding,
        ctx.enclosing_owner,
    )
}

/// Attach cast and satisfies expression operator seams.
fn attach_expression_operator_cast_and_satisfies(
    ctx: &mut ExpressionOperatorDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_expression_operator_cast_and_satisfies(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.owners.preceding,
        ctx.owners.following,
        ctx.enclosing_owner,
    )
}

/// Attach elementwise expression operator seams.
fn attach_expression_operator_elementwise(
    ctx: &mut ExpressionOperatorDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_expression_operator_elementwise(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.preceding,
        ctx.owners.following,
    )
}

/// Attach multiline `as const` expression operator seams.
fn attach_expression_operator_as_const_multiline(
    ctx: &mut ExpressionOperatorDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_comment_expression_operator_as_const_multiline(
        ctx.tree,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
    )
}

/// Ordered expression-operator seam handlers.
const EXPRESSION_OPERATOR_HANDLERS: &[ExpressionOperatorHandler] = &[
    attach_expression_operator_assignment_and_grouping,
    attach_expression_operator_cast_and_satisfies,
    attach_expression_operator_elementwise,
    attach_expression_operator_as_const_multiline,
];

/// Resolve expression operator seam comment rules.
pub(crate) fn try_attach_comment_expression_operator(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let enclosing_owner = comment_enclosing_owner(context, enclosing_owner_cache);
    let mut ctx = ExpressionOperatorDispatchContext {
        tree,
        parents,
        seam_ctx: context,
        seam,
        enclosing_owner_cache,
        owners,
        enclosing_owner,
    };

    run_expression_operator_handlers(&mut ctx, EXPRESSION_OPERATOR_HANDLERS)
}
