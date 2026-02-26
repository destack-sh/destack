use destack_ast::{
    AnnotationPosition, Declaration, Expression, FunctionKind, IfKind, LocalNodeId,
    NodeParentIndex, NodeTree, NodeType, TokenSpan, TokenType, WhileKind,
};

use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentSeamContext, CommentSeamData,
    previous_non_newline_token_index,
};
use super::facts::{next_non_trivia_token_index, next_non_trivia_token_type, token_type_is_trivia};
use super::ownership::{
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_to_nearest_statement_boundary,
    promote_owner_to_node_type_ancestor,
};
use crate::DestackFormatContext;

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

/// Return whether one token type can start one semicolon-guarded statement head.
#[inline]
pub(crate) fn token_type_is_semicolon_guard_head(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenBracket | TokenType::OpenParenthesis | TokenType::Add | TokenType::Subtract
    )
}

/// Return whether one semicolon guard target needs boundary continuation formatting.
#[inline]
pub(crate) fn token_type_is_semicolon_guard_boundary_continuation(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenBracket | TokenType::OpenParenthesis
    )
}

/// Return whether one token type can be one guard target after semicolon.
#[inline]
pub(crate) fn token_type_is_semicolon_guard_target(token_type: TokenType) -> bool {
    token_type_is_semicolon_guard_head(token_type)
}

/// Return whether one semicolon seam should preserve the preceding boundary continuation.
#[inline]
pub(crate) fn semicolon_guard_seam_prefers_preceding_boundary_continuation(
    seam: SemicolonGuardCommentSeam,
) -> bool {
    matches!(
        seam,
        SemicolonGuardCommentSeam::AfterComment { target_type }
            if token_type_is_semicolon_guard_boundary_continuation(target_type)
    )
}

/// Return whether one semicolon guard seam targets one `[` array guard head.
#[inline]
pub(crate) fn semicolon_guard_targets_array_literal(
    seam: SemicolonGuardCommentSeam,
    token_before_is_semicolon: bool,
    token_after_type: Option<TokenType>,
) -> bool {
    (token_after_type == Some(TokenType::Semicolon)
        && seam.after_comment_target_type() == Some(TokenType::OpenBracket))
        || (token_before_is_semicolon && token_after_type == Some(TokenType::OpenBracket))
}

/// Classify one semicolon guard seam around one own-line comment.
pub(crate) fn classify_semicolon_guard_comment_seam(
    semantic_tokens: &[TokenSpan],
    token_before_type: Option<TokenType>,
    token_after_type: Option<TokenType>,
    token_after_index: Option<usize>,
) -> SemicolonGuardCommentSeam {
    if token_before_type == Some(TokenType::Semicolon) {
        let target_type = token_after_type
            .filter(|token_type| token_type_is_semicolon_guard_head(*token_type))
            .or_else(|| {
                let token_after_is_trivia = token_after_type.is_none_or(token_type_is_trivia);
                if !token_after_is_trivia {
                    return None;
                }

                token_after_index
                    .and_then(|index| next_non_trivia_token_type(semantic_tokens, index))
                    .filter(|token_type| token_type_is_semicolon_guard_head(*token_type))
            });

        if let Some(target_type) = target_type {
            return SemicolonGuardCommentSeam::BeforeComment { target_type };
        }
    }

    if token_after_type == Some(TokenType::Semicolon)
        && let Some(token_after_index) = token_after_index
        && let Some(target_type) = next_non_trivia_token_type(semantic_tokens, token_after_index)
        && token_type_is_semicolon_guard_target(target_type)
    {
        return SemicolonGuardCommentSeam::AfterComment { target_type };
    }

    SemicolonGuardCommentSeam::None
}

/// Return one preceding owner with one previous non-newline token fallback owner.
pub(crate) fn preceding_owner_with_non_newline_token_fallback(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    preceding_owner: Option<u32>,
) -> Option<u32> {
    let fallback_owner = context
        .token_before
        .and_then(|token_index| {
            previous_non_newline_token_index(context.semantic_tokens, token_index)
        })
        .and_then(|token_index| context.semantic_tokens.get(token_index))
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));

    preceding_owner.or(fallback_owner)
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

/// Return whether one owner is one top-level statement owner.
fn owner_is_top_level_statement(tree: &NodeTree, parents: &NodeParentIndex, owner: u32) -> bool {
    let block_ancestor = promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Block);
    let owner_is_statement = match tree.get_node_type(owner) {
        NodeType::Declaration => true,
        NodeType::Expression => {
            let mut current_expression_id = LocalNodeId::<Expression>::new(owner);
            loop {
                match tree.get(current_expression_id) {
                    Expression::Statement(_) => break true,
                    Expression::Declaration(_) => break true,
                    Expression::Parenthesized { expression } => {
                        current_expression_id = *expression;
                    }
                    _ => break false,
                }
            }
        }
        _ => false,
    };

    owner_is_statement && block_ancestor.is_none()
}

/// Return one comment column for one trivia span.
fn comment_column(context: &CommentSeamContext<'_>) -> u32 {
    let comment_start = context
        .semantic_tokens
        .iter()
        .find(|token| {
            token.span.start >= context.trivia.span.start
                && token.span.end <= context.trivia.span.end
                && matches!(
                    token.token.ty,
                    TokenType::LineComment
                        | TokenType::BlockComment
                        | TokenType::DocLineComment
                        | TokenType::DocBlockComment
                )
        })
        .map_or(context.trivia.span.start, |token| token.span.start);

    context
        .file
        .get_position(comment_start)
        .map_or(1, |(_, column)| column)
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
        context.semantic_tokens,
        seam.token_before_type,
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

    // preserve preceding boundary continuation when the seam is one indented `comment ; [` or `comment ; (`
    let should_preserve_preceding_boundary_continuation = semicolon_guard_seam.is_after_comment()
        && semicolon_guard_seam_prefers_preceding_boundary_continuation(semicolon_guard_seam)
        && comment_column(context) > 1
        && preceding_statement_owner
            .is_some_and(|owner| owner_is_top_level_statement(tree, parents, owner));
    if should_preserve_preceding_boundary_continuation
        && let Some(target_owner) = preceding_statement_owner
    {
        let target_owner =
            normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    // otherwise guard comments belong to the guarded following expression
    if let Some(target_owner) = following_expression_owner {
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    // if no following-side owner is available for one after-comment seam, skip fallback ownership
    if semicolon_guard_seam.is_after_comment() && preceding_statement_owner.is_none() {
        return None;
    }

    if let Some(target_owner) = preceding_statement_owner {
        let target_owner =
            normalize_owner_with_shared_end(tree, parents, target_owner, token_before_span);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Return whether one block expression needs a trailing statement terminator.
pub(crate) fn expression_needs_statement_terminator(
    context: &DestackFormatContext<'_>,
    expression: &Expression,
    is_expression_context_tail: bool,
) -> bool {
    if is_expression_context_tail {
        return false;
    }

    matches!(
        expression,
        Expression::Import { .. } | Expression::Let { .. } | Expression::Using { .. }
    ) || matches!(
        expression,
        Expression::While {
            kind: WhileKind::DoWhile,
            ..
        }
    ) || matches!(
        expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function {
                    descriptor,
                    signature,
                    ..
                }
                if descriptor.name.is_none() && signature.kind == FunctionKind::Lambda
            )
    ) || (!matches!(expression, Expression::Statement(_))
        && !matches!(expression, Expression::Stub | Expression::Error)
        && !expression.ends_statement_on_newline())
}

/// Return whether one statement wrapper should keep its trailing semicolon.
pub(crate) fn statement_wrapper_needs_semicolon(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);
    if let Expression::Statement(inner_expression_id) = expression {
        return statement_wrapper_needs_semicolon(context, *inner_expression_id);
    }

    if matches!(
        expression,
        Expression::Declaration(_) | Expression::Block(_)
    ) {
        return false;
    }

    if matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        } | Expression::While {
            kind: WhileKind::While,
            ..
        } | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Loop { .. }
            | Expression::Try { .. }
            | Expression::Match { .. }
            | Expression::Labelled { .. }
    ) {
        return false;
    }

    true
}
