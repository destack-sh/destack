use destack_ast::{
    AnnotationPosition, Block, BlockFormat, Expression, LocalNodeId, NodeParentIndex, NodeTree,
    NodeType, TokenSpan, TokenType,
};
use destack_source::Span;

use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentSeamContext, CommentSeamData,
    is_close_delimiter_token, previous_non_newline_token_index,
};
use super::facts::{
    next_non_trivia_token_index, token_type_is_comment_trivia, token_type_is_trivia,
};
use super::ownership::{
    find_preferred_owner_starting_at, find_smallest_owner_enclosing_token,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_end, promote_owner_to_nearest_statement_boundary,
    promote_owner_to_node_type_ancestor,
};
use super::terminator::owner_starts_with_asi_hazard;
use crate::{Annotation, DestackFormatContext};

/// One normalized semicolon guard seam classification for own-line comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemicolonGuardCommentSeam {
    /// No semicolon guard seam was detected.
    None,
    /// The comment is between one semicolon and one guard head token.
    BeforeComment {
        /// The guard head token after the comment.
        target_type: TokenType,
    },
    /// The comment is followed by one semicolon and one guard target token.
    AfterComment {
        /// The first non-trivia guard target token after the semicolon.
        target_type: TokenType,
    },
}

impl SemicolonGuardCommentSeam {
    /// Return whether this seam classification has one semicolon guard shape.
    #[inline]
    pub(crate) fn has_guard_shape(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Return whether the comment appears after one semicolon in this seam.
    #[inline]
    pub(crate) fn is_after_comment(self) -> bool {
        matches!(self, Self::AfterComment { .. })
    }

    /// Return one guard target token type when the seam is one after-comment seam.
    #[inline]
    pub(crate) fn after_comment_target_type(self) -> Option<TokenType> {
        match self {
            Self::AfterComment { target_type } => Some(target_type),
            Self::None | Self::BeforeComment { .. } => None,
        }
    }
}

/// Return one token index and token type for one before-comment semicolon guard seam.
fn before_comment_semicolon_guard_target(
    semantic_tokens: &[TokenSpan],
    token_after_type: Option<TokenType>,
    token_after_index: Option<usize>,
) -> Option<(usize, TokenType)> {
    if token_after_type.is_some_and(token_type_is_trivia) {
        return token_after_index
            .and_then(|index| next_non_trivia_token_index(semantic_tokens, index))
            .and_then(|index| {
                semantic_tokens
                    .get(index)
                    .map(|token| (index, token.token.ty))
            });
    }

    token_after_index.zip(token_after_type)
}

/// Return one token index and token type for one after-comment semicolon guard seam.
fn after_comment_semicolon_guard_target(
    semantic_tokens: &[TokenSpan],
    token_after_index: Option<usize>,
) -> Option<(usize, TokenType)> {
    token_after_index
        .and_then(|index| next_non_trivia_token_index(semantic_tokens, index))
        .and_then(|index| {
            semantic_tokens
                .get(index)
                .map(|token| (index, token.token.ty))
        })
}

/// Return whether one semicolon token appears at one logical line start.
fn semicolon_token_is_line_leading(semantic_tokens: &[TokenSpan], semicolon_index: usize) -> bool {
    if semicolon_index == 0 {
        return true;
    }

    let mut cursor = semicolon_index;
    while cursor > 0 {
        cursor -= 1;
        let token_type = semantic_tokens[cursor].token.ty;

        if token_type == TokenType::Whitespace {
            continue;
        }

        if token_type == TokenType::Newline {
            return true;
        }

        if token_type_is_comment_trivia(token_type) {
            continue;
        }

        return false;
    }

    true
}

/// One semicolon seam side relative to one comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemicolonSeamSide {
    /// The semicolon appears before the comment.
    BeforeComment,
    /// The semicolon appears after the comment.
    AfterComment,
}

impl SemicolonSeamSide {
    /// Build one guard seam result for this seam side.
    #[inline]
    fn to_guard_seam(self, target_type: TokenType) -> SemicolonGuardCommentSeam {
        match self {
            Self::BeforeComment => SemicolonGuardCommentSeam::BeforeComment { target_type },
            Self::AfterComment => SemicolonGuardCommentSeam::AfterComment { target_type },
        }
    }
}

/// Return whether one semicolon on one seam side is line-leading.
fn seam_side_has_line_leading_semicolon(
    semantic_tokens: &[TokenSpan],
    semicolon_type: Option<TokenType>,
    semicolon_index: Option<usize>,
) -> bool {
    if semicolon_type != Some(TokenType::Semicolon) {
        return false;
    }

    semicolon_index.is_some_and(|index| semicolon_token_is_line_leading(semantic_tokens, index))
}

/// Return whether one seam has one line-leading semicolon before its comment.
pub(crate) fn seam_has_line_leading_semicolon_before_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    seam_side_has_line_leading_semicolon(
        context.semantic_tokens,
        seam.token_before_type,
        context.token_before,
    )
}

/// Return whether one seam has one line-leading semicolon after its comment.
pub(crate) fn seam_has_line_leading_semicolon_after_comment(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    seam_side_has_line_leading_semicolon(
        context.semantic_tokens,
        seam.token_after_type,
        context.token_after,
    )
}

/// Return whether one semicolon guard target owner starts with one ASI hazard.
fn semicolon_guard_target_owner_starts_with_asi_hazard(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner: u32,
) -> Option<bool> {
    let statement_owner = promote_owner_to_nearest_statement_boundary(tree, parents, owner);
    if let Some(is_asi_hazard) = owner_starts_with_asi_hazard(tree, statement_owner) {
        return Some(is_asi_hazard);
    }

    let expression_owner =
        promote_owner_to_node_type_ancestor(tree, parents, statement_owner, NodeType::Expression);
    if let Some(expression_owner) = expression_owner {
        return owner_starts_with_asi_hazard(tree, expression_owner);
    }

    let declaration_owner =
        promote_owner_to_node_type_ancestor(tree, parents, statement_owner, NodeType::Declaration);
    if let Some(declaration_owner) = declaration_owner {
        return owner_starts_with_asi_hazard(tree, declaration_owner);
    }

    None
}

/// Return whether one semicolon guard seam has one structural ASI hazard target.
fn semicolon_guard_target_is_asi_hazard(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    semantic_tokens: &[TokenSpan],
    target_token_index: usize,
) -> bool {
    let target_owner_from_token = semantic_tokens.get(target_token_index).and_then(|token| {
        find_preferred_owner_starting_at(tree, token.span)
            .or_else(|| find_smallest_owner_enclosing_token(tree, token.span))
    });

    target_owner_from_token.is_some_and(|owner| {
        semicolon_guard_target_owner_starts_with_asi_hazard(tree, parents, owner) == Some(true)
    })
}

/// Return whether one annotation needs continuation indentation for semicolon-guard seams.
pub(crate) fn annotation_needs_semicolon_guard_continuation_indent(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
    is_slash_comment: bool,
    starts_on_own_line: bool,
) -> bool {
    if position != AnnotationPosition::LinePostfixBoundary || !is_slash_comment {
        return false;
    }

    if !starts_on_own_line {
        return false;
    }

    if !context.annotation_starts_indented(annotation_id) {
        return false;
    }

    context.annotation_semicolon_guard_target_token_type(annotation_id)
        == Some(TokenType::OpenParenthesis)
}

/// Return whether one semicolon guard target needs boundary continuation formatting.
#[inline]
pub(crate) fn token_type_is_semicolon_guard_boundary_continuation(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenBracket | TokenType::OpenParenthesis
    )
}

/// Classify one semicolon guard seam around one own-line comment.
pub(crate) fn classify_semicolon_guard_comment_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    semantic_tokens: &[TokenSpan],
    token_before_type: Option<TokenType>,
    token_before_index: Option<usize>,
    token_after_type: Option<TokenType>,
    token_after_index: Option<usize>,
) -> SemicolonGuardCommentSeam {
    let before_comment_target =
        before_comment_semicolon_guard_target(semantic_tokens, token_after_type, token_after_index);
    if let Some(seam) = classify_semicolon_guard_side(
        tree,
        parents,
        semantic_tokens,
        SemicolonSeamSide::BeforeComment,
        token_before_type,
        token_before_index,
        before_comment_target,
    ) {
        return seam;
    }

    let after_comment_target =
        after_comment_semicolon_guard_target(semantic_tokens, token_after_index);
    if let Some(seam) = classify_semicolon_guard_side(
        tree,
        parents,
        semantic_tokens,
        SemicolonSeamSide::AfterComment,
        token_after_type,
        token_after_index,
        after_comment_target,
    ) {
        return seam;
    }

    SemicolonGuardCommentSeam::None
}

/// Classify one semicolon guard seam shape for one seam side.
fn classify_semicolon_guard_side(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    semantic_tokens: &[TokenSpan],
    seam_side: SemicolonSeamSide,
    semicolon_type: Option<TokenType>,
    semicolon_index: Option<usize>,
    target: Option<(usize, TokenType)>,
) -> Option<SemicolonGuardCommentSeam> {
    if semicolon_type != Some(TokenType::Semicolon) {
        return None;
    }

    let (target_token_index, target_type) = target?;
    let semicolon_is_line_leading =
        seam_side_has_line_leading_semicolon(semantic_tokens, semicolon_type, semicolon_index);
    let non_leading_semicolon_can_guard =
        token_type_is_semicolon_guard_boundary_continuation(target_type);
    if !semicolon_is_line_leading && !non_leading_semicolon_can_guard {
        return None;
    }

    let target_is_asi_hazard =
        semicolon_guard_target_is_asi_hazard(tree, parents, semantic_tokens, target_token_index);
    if !target_is_asi_hazard {
        return None;
    }

    Some(seam_side.to_guard_seam(target_type))
}

/// Return one empty-statement block owner when one seam appears before its semicolon.
fn empty_statement_body_owner_before_semicolon(
    tree: &NodeTree,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<u32> {
    if !seam.token_after_is(TokenType::Semicolon) {
        return None;
    }

    let following_owner = following_owner?;
    let block_owner = if tree.get_node_type(following_owner) == NodeType::Block {
        following_owner
    } else if tree.get_node_type(following_owner) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(following_owner);
        match tree.get(expression_id) {
            Expression::Block(block_id) => block_id.id,
            _ => return None,
        }
    } else {
        return None;
    };

    let block_id = LocalNodeId::<Block>::new(block_owner);
    let block = tree.get(block_id);
    if block.format != BlockFormat::Implicit || !block.expressions.is_empty() {
        return None;
    }

    Some(block_owner)
}

/// Attach one seam comment that appears before one empty-statement body semicolon.
pub(crate) fn try_attach_comment_before_empty_statement_semicolon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let target_owner = empty_statement_body_owner_before_semicolon(tree, seam, following_owner)?;

    // line comments between a control head and empty statement semicolon
    // stay on the preceding control statement boundary
    if seam.comment_is_line {
        let boundary_owner = context
            .token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(preceding_owner)
            .or_else(|| {
                context
                    .token_before
                    .and_then(|token_index| {
                        previous_non_newline_token_index(context.semantic_tokens, token_index)
                    })
                    .and_then(|token_index| context.semantic_tokens.get(token_index))
                    .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            })
            .or_else(|| {
                let target_statement_owner =
                    promote_owner_to_nearest_statement_boundary(tree, parents, target_owner);
                (target_statement_owner != target_owner).then_some(target_statement_owner)
            })
            .or_else(|| {
                parents
                    .get_by_id(target_owner)
                    .map(|owner| promote_owner_to_nearest_statement_boundary(tree, parents, owner))
            })
            .unwrap_or(target_owner);
        let boundary_owner =
            promote_owner_to_nearest_statement_boundary(tree, parents, boundary_owner);
        return Some((
            Some(boundary_owner),
            AnnotationPosition::LinePostfixBoundary,
        ));
    }

    let position = AnnotationPosition::BlockPrefix;

    Some((Some(target_owner), position))
}

/// Attach one same-line comment before one semicolon to one preceding expression boundary.
pub(crate) fn attach_inline_comment_before_semicolon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    semicolon_after_is_line_leading: bool,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
    is_same_line_comment: bool,
) -> Option<CommentAttachment> {
    if !is_same_line_comment
        || !seam.token_after_is(TokenType::Semicolon)
        || seam.token_before_is(TokenType::Semicolon)
    {
        return None;
    }

    let token_before_owner =
        token_before_span.and_then(|span| find_smallest_owner_enclosing_token(tree, span));
    if semicolon_after_is_line_leading {
        let target_owner = preceding_owner.or(token_before_owner)?;
        let target_owner = promote_owner_to_nearest_statement_boundary(tree, parents, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    let target_owner = if seam.token_before_type.is_some_and(is_close_delimiter_token) {
        token_before_owner.or(preceding_owner)
    } else {
        preceding_owner.or(token_before_owner)
    }?;
    let target_owner = token_before_span
        .map(|span| promote_owner_by_shared_end(tree, parents, target_owner, span.end))
        .unwrap_or(target_owner);
    let target_owner = if tree.get_node_type(target_owner) == NodeType::Expression {
        target_owner
    } else if let Some(expression_owner) =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Expression)
    {
        expression_owner
    } else {
        promote_owner_to_nearest_statement_boundary(tree, parents, target_owner)
    };

    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Attach one same-line comment after one semicolon-terminated statement.
pub(crate) fn attach_after_semicolon_terminated_statement_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    semicolon_is_line_leading: bool,
    preceding_owner_with_semicolon_fallback: Option<u32>,
    token_before_span: Option<Span>,
    is_same_line_comment: bool,
) -> Option<CommentAttachment> {
    if !seam.token_before_is(TokenType::Semicolon) || !is_same_line_comment {
        return None;
    }

    // line-leading semicolons are standalone empty statements, not statement terminators
    if semicolon_is_line_leading {
        return None;
    }

    let target_owner = preceding_owner_with_semicolon_fallback?;
    let target_owner =
        normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
    if let Some(member_owner) =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Member)
    {
        let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
        return Some((Some(member_owner), AnnotationPosition::LinePostfixBoundary));
    }

    let target_owner = promote_owner_to_nearest_statement_boundary(tree, parents, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Attach one own-line comment before one member semicolon seam.
pub(crate) fn attach_own_line_comment_before_member_semicolon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if !seam.token_after_is(TokenType::Semicolon) {
        return None;
    }

    let target_owner = preceding_owner?;
    let target_owner =
        normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
    let member_owner =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Member)?;
    let member_owner = normalize_formatter_trivia_target_owner(tree, member_owner);
    Some((Some(member_owner), AnnotationPosition::BlockPostfix))
}

/// Return one following expression owner for one semicolon guard seam.
fn semicolon_guard_following_expression_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: SemicolonGuardCommentSeam,
    token_after_span: Option<TokenSpan>,
    following_owner: Option<u32>,
) -> Option<u32> {
    let following_owner = context
        .token_after
        .and_then(|token_index| {
            if seam.is_after_comment() {
                next_non_trivia_token_index(context.semantic_tokens, token_index)
            } else {
                None
            }
        })
        .and_then(|token_index| context.semantic_tokens.get(token_index))
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        .or_else(|| {
            token_after_span.and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        })
        .or(following_owner)
        .map(|target_owner| {
            promote_owner_to_nearest_statement_boundary(tree, parents, target_owner)
        })
        .and_then(|target_owner| {
            if tree.get_node_type(target_owner) == NodeType::Expression {
                Some(target_owner)
            } else {
                promote_owner_to_node_type_ancestor(
                    tree,
                    parents,
                    target_owner,
                    NodeType::Expression,
                )
            }
        })?;

    Some(following_owner)
}

/// Attach one non-own-line comment between `;` and one ASI guard head.
pub(crate) fn attach_before_comment_semicolon_guard_head(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline {
        return None;
    }

    let semicolon_guard_seam = classify_semicolon_guard_comment_seam(
        tree,
        parents,
        context.semantic_tokens,
        seam.token_before_type,
        context.token_before,
        seam.token_after_type,
        context.token_after,
    );
    if !matches!(
        semicolon_guard_seam,
        SemicolonGuardCommentSeam::BeforeComment { .. }
    ) {
        return None;
    }

    let target_owner = semicolon_guard_following_expression_owner(
        tree,
        parents,
        context,
        semicolon_guard_seam,
        context.token_after_span,
        following_owner,
    )?;
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    let position = if seam.comment_is_line || seam.has_trailing_newline {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };

    Some((Some(target_owner), position))
}

/// Return one preceding statement owner for one semicolon guard seam.
fn semicolon_guard_preceding_statement_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<u32> {
    let preceding_owner_from_previous_non_newline_token =
        if seam.token_before_is(TokenType::Semicolon) {
            context
                .token_before
                .and_then(|token_index| {
                    previous_non_newline_token_index(context.semantic_tokens, token_index)
                })
                .and_then(|token_index| context.semantic_tokens.get(token_index))
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        } else {
            None
        };
    let preceding_owner_from_token_before = context
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let preceding_owner_candidate = if seam.token_before_is(TokenType::Semicolon) {
        preceding_owner_from_previous_non_newline_token
            .or(owners.preceding)
            .or(preceding_owner_from_token_before)
    } else {
        owners
            .preceding
            .or(preceding_owner_from_token_before)
            .or(preceding_owner_from_previous_non_newline_token)
    };

    preceding_owner_candidate
        .map(|owner| promote_owner_to_nearest_statement_boundary(tree, parents, owner))
}

/// Return whether one owner appears inside one block ancestor.
#[inline]
fn owner_has_block_ancestor(tree: &NodeTree, parents: &NodeParentIndex, owner: u32) -> bool {
    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Block).is_some()
}

/// Attach one own-line semicolon-guard comment across both seam shapes.
pub(crate) fn attach_semicolon_guard_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline || !seam.comment_is_line {
        return None;
    }

    // classify the semicolon guard seam once
    let semicolon_guard_seam = classify_semicolon_guard_comment_seam(
        tree,
        parents,
        context.semantic_tokens,
        seam.token_before_type,
        context.token_before,
        seam.token_after_type,
        context.token_after,
    );
    if !semicolon_guard_seam.has_guard_shape() {
        return None;
    }

    let token_after_span = context.token_after_span;
    let token_before_span = context.token_before_span.map(|token| token.span);
    let following_expression_owner = semicolon_guard_following_expression_owner(
        tree,
        parents,
        context,
        semicolon_guard_seam,
        token_after_span,
        owners.following,
    );
    let preceding_statement_owner =
        semicolon_guard_preceding_statement_owner(tree, parents, context, seam, owners);

    // after-comment array guard seams belong to the guarded expression:
    // `comment ; [ ... ]`
    if semicolon_guard_seam.after_comment_target_type() == Some(TokenType::OpenBracket)
        && let Some(target_owner) = following_expression_owner
    {
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    // nested `comment ; ( ... )` seams format as following-expression prefixes
    if semicolon_guard_seam.after_comment_target_type() == Some(TokenType::OpenParenthesis)
        && preceding_statement_owner
            .is_some_and(|target_owner| owner_has_block_ancestor(tree, parents, target_owner))
        && let Some(target_owner) = following_expression_owner
    {
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    // non-array after-comment guard seams stay on the preceding statement boundary:
    // `comment ; ( ... )`, `comment ; +value`, `comment ; -value`
    if semicolon_guard_seam.is_after_comment() {
        if let Some(target_owner) = preceding_statement_owner {
            let target_owner =
                normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
            return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
        }

        if let Some(target_owner) = following_expression_owner {
            let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
            return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
        }

        return None;
    }

    // before-comment seams belong to the guarded following expression:
    // `; comment <asi-hazard>`
    if let Some(target_owner) = following_expression_owner {
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    // fallback to preceding statement boundary when no following owner is available
    if let Some(target_owner) = preceding_statement_owner {
        let target_owner =
            normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Attach one own-line line comment before one statement semicolon seam.
pub(crate) fn attach_own_line_comment_before_statement_semicolon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
    preceding_owner: Option<u32>,
    following_owner_with_token_after_fallback: Option<u32>,
    token_before_span: Option<Span>,
) -> Option<CommentAttachment> {
    if !seam.comment_is_line || !seam.token_after_is(TokenType::Semicolon) {
        return None;
    }

    if let Some(attachment) =
        attach_semicolon_guard_own_line_comment(tree, parents, context, seam, owners)
    {
        return Some(attachment);
    }

    let target_owner = following_owner_with_token_after_fallback
        .map(|target_owner| {
            promote_owner_to_nearest_statement_boundary(tree, parents, target_owner)
        })
        .filter(|target_owner| tree.get_node_type(*target_owner) != NodeType::Block)
        .map(|target_owner| normalize_formatter_trivia_target_owner(tree, target_owner));
    if let Some(target_owner) = target_owner {
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    let target_owner = preceding_owner.or_else(|| {
        token_before_span.and_then(|span| find_smallest_owner_enclosing_token(tree, span))
    })?;
    let target_owner =
        normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}
