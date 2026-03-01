use destack_ast::{
    AnnotationPosition, Blank, Expression, Keyword, LocalNodeId, NodeParentIndex, NodeTree,
    NodeType, TokenSpan, TokenType,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;
use rustc_hash::FxHashMap;

use crate::{DestackFormatter, FormatNode};

use super::attachment::{
    FormatterTriviaOwnerIndex, FormatterTriviaSeamIndex, decode_token_index, encode_trivia_seam,
};
use super::boundary::{CommentSeamKeyword, comment_seam_keyword, previous_non_newline_token_index};
use super::facts::{
    next_non_trivia_token_index, previous_non_trivia_token_index, token_type_is_comment_trivia,
    token_type_is_trivia,
};
use super::ownership::{
    find_owner_at_or_after_token_with_node_type, find_smallest_owner_enclosing_range,
    find_smallest_owner_enclosing_token, lowest_common_owner_ancestor,
    promote_owner_by_shared_start, promote_owner_to_declaration_ancestor,
    promote_owner_to_nearest_statement_boundary, promote_owner_to_node_type_ancestor,
    promote_owner_to_statement_boundary,
};
use super::semicolon::{
    SemicolonGuardCommentSeam, classify_semicolon_guard_comment_seam,
    semicolon_guard_targets_array_literal,
};

impl<'ast> FormatNode<'ast, Blank> for Blank {
    /// Format one blank annotation node.
    fn format_node(
        &self,
        _node_id: LocalNodeId<Blank>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // reduce any number of blank lines to a single one
        write!(f, [empty_line()])?;
        Ok(())
    }
}

/// Return one blank infix attachment.
#[inline]
fn blank_infix_attachment() -> (Option<u32>, AnnotationPosition) {
    (None, AnnotationPosition::BlockInfix)
}

/// Normalize one blank-trivia owner while preserving declaration expression boundaries.
#[inline]
fn normalize_blank_target_owner(tree: &NodeTree, owner_id: u32) -> u32 {
    let mut current_id = owner_id;

    loop {
        if tree.get_node_type(current_id) != NodeType::Expression {
            return current_id;
        }

        let expression_id = LocalNodeId::<Expression>::new(current_id);
        match tree.get(expression_id) {
            // statement wrappers are transparent for blank ownership
            Expression::Statement(expression) => {
                current_id = expression.id;
            }
            // declaration wrappers are transparent: declaration formatting owns declaration prefixes
            Expression::Declaration(declaration_id) => {
                return declaration_id.id;
            }
            // parenthesized wrappers around declaration expressions are transparent
            Expression::Parenthesized { expression }
                if matches!(tree.get(*expression), Expression::Declaration(_)) =>
            {
                current_id = expression.id;
            }
            // keep declaration expressions as statement-level blank owners
            _ => {
                return current_id;
            }
        }
    }
}

/// Return one normalized block-prefix attachment.
#[inline]
fn block_prefix_attachment(tree: &NodeTree, target_node: u32) -> (Option<u32>, AnnotationPosition) {
    let target_node = normalize_blank_target_owner(tree, target_node);
    (Some(target_node), AnnotationPosition::BlockPrefix)
}

/// Return one normalized block-postfix attachment.
#[inline]
fn block_postfix_attachment(
    tree: &NodeTree,
    target_node: u32,
) -> (Option<u32>, AnnotationPosition) {
    let target_node = normalize_blank_target_owner(tree, target_node);
    (Some(target_node), AnnotationPosition::BlockPostfix)
}

/// Return one statement-boundary owner before a semicolon seam.
fn owner_before_semicolon(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    semantic_tokens: &[TokenSpan],
    token_before: Option<usize>,
    token_before_is_semicolon: bool,
    preceding_token_owner: Option<u32>,
    preceding_owner: Option<u32>,
) -> Option<u32> {
    let owner = if token_before_is_semicolon {
        token_before
            .and_then(|token_index| previous_non_newline_token_index(semantic_tokens, token_index))
            .and_then(|token_index| semantic_tokens.get(token_index))
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(preceding_owner)
    } else {
        preceding_token_owner.or(preceding_owner)
    };

    owner.map(|owner_id| promote_owner_to_statement_boundary(tree, parents, owner_id))
}

/// Return whether one seam should be treated as one top-level statement spacing gap.
fn seam_is_top_level_statement_spacing_gap(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
    seam_has_comment: bool,
    seam_has_line_comment: bool,
    token_before_is_statement_end: bool,
    token_before_is_semicolon: bool,
) -> bool {
    if seam_has_comment || seam_has_line_comment || !token_before_is_statement_end {
        return false;
    }

    if token_before_is_semicolon
        && preceding_token_owner
            .zip(preceding_owner)
            .is_some_and(|(token_owner, preceding_owner)| token_owner != preceding_owner)
    {
        return false;
    }

    let (Some(preceding_owner), Some(following_owner)) = (preceding_owner, following_owner) else {
        return false;
    };

    let preceding_statement_owner =
        promote_owner_to_statement_boundary(tree, parents, preceding_owner);
    let following_statement_owner =
        promote_owner_to_statement_boundary(tree, parents, following_owner);
    if preceding_statement_owner == following_statement_owner {
        return false;
    }

    let following_is_top_level =
        promote_owner_to_node_type_ancestor(tree, parents, following_owner, NodeType::Block)
            .is_none();
    if !following_is_top_level {
        return false;
    }

    let preceding_type = tree.get_node_type(preceding_owner);
    let following_type = tree.get_node_type(following_owner);

    matches!(preceding_type, NodeType::Expression | NodeType::Declaration)
        && matches!(following_type, NodeType::Expression | NodeType::Declaration)
}

/// Return one start owner at one semantic token index.
fn start_owner_at_token(
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: Option<usize>,
) -> Option<u32> {
    token_index
        .and_then(|index| {
            owner_index
                .owner_start_by_token
                .get(index)
                .and_then(|owner| *owner)
        })
        .or_else(|| {
            token_index.and_then(|index| {
                owner_index
                    .nearest_owner_start_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
        })
}

/// Return one end owner at one semantic token index.
fn end_owner_at_token(
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: Option<usize>,
) -> Option<u32> {
    token_index
        .and_then(|index| {
            owner_index
                .owner_end_by_token
                .get(index)
                .and_then(|owner| *owner)
        })
        .or_else(|| {
            token_index.and_then(|index| {
                owner_index
                    .nearest_owner_end_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
        })
}

/// Return normalized seam token indexes for one blank trivia boundary.
fn normalized_blank_seam_token_indexes(
    semantic_tokens: &[TokenSpan],
    trivia: destack_ast::BlankTrivia,
) -> (
    Option<usize>,
    Option<usize>,
    Option<usize>,
    Option<usize>,
    u64,
    u64,
) {
    let raw_token_before = decode_token_index(trivia.boundary.token_before);
    let raw_token_after = decode_token_index(trivia.boundary.token_after);

    let normalized_token_before = raw_token_before.and_then(|token_index| {
        let token_type = semantic_tokens
            .get(token_index)
            .map(|token| token.token.ty)?;
        if token_type_is_trivia(token_type) {
            previous_non_trivia_token_index(semantic_tokens, token_index)
        } else {
            Some(token_index)
        }
    });
    let normalized_token_after = raw_token_after.and_then(|token_index| {
        let token_type = semantic_tokens
            .get(token_index)
            .map(|token| token.token.ty)?;
        if token_type_is_trivia(token_type) {
            next_non_trivia_token_index(semantic_tokens, token_index)
        } else {
            Some(token_index)
        }
    });

    let seam = encode_trivia_seam(trivia.boundary.token_before, trivia.boundary.token_after);
    let normalized_seam = encode_trivia_seam(
        normalized_token_before.map_or(u32::MAX, |index| index as u32),
        normalized_token_after.map_or(u32::MAX, |index| index as u32),
    );

    (
        raw_token_before,
        raw_token_after,
        normalized_token_before,
        normalized_token_after,
        seam,
        normalized_seam,
    )
}

/// Return whether one owner has one `if` statement expression ancestor.
fn owner_has_if_statement_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner: Option<u32>,
) -> bool {
    let mut current_owner = owner;
    while let Some(owner_id) = current_owner {
        if tree.get_node_type(owner_id) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(owner_id)),
                Expression::If {
                    kind: destack_ast::IfKind::If,
                    ..
                }
            )
        {
            return true;
        }

        current_owner = parents.get_by_id(owner_id);
    }

    false
}

/// Return comment trivia facts across one token index range.
fn comment_trivia_summary_in_token_range(
    semantic_tokens: &[TokenSpan],
    first_index: Option<usize>,
    second_index: Option<usize>,
) -> (bool, bool, Option<u32>) {
    let (Some(first_index), Some(second_index)) = (first_index, second_index) else {
        return (false, false, None);
    };
    let (start_index, end_index) = if first_index <= second_index {
        (first_index, second_index)
    } else {
        (second_index, first_index)
    };

    let mut has_comment = false;
    let mut has_line_comment = false;
    let mut first_comment_start = None;

    for token in semantic_tokens
        .iter()
        .skip(start_index)
        .take(end_index - start_index + 1)
    {
        let token_type = token.token.ty;
        if !matches!(
            token_type,
            TokenType::LineComment
                | TokenType::BlockComment
                | TokenType::DocLineComment
                | TokenType::DocBlockComment
        ) {
            continue;
        }

        has_comment = true;
        first_comment_start = first_comment_start.or(Some(token.span.start));
        if matches!(
            token_type,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            has_line_comment = true;
        }
    }

    (has_comment, has_line_comment, first_comment_start)
}

/// One normalized token-resolution snapshot for one blank seam.
struct BlankSeamTokenState {
    raw_token_after: Option<usize>,
    token_before: Option<usize>,
    token_after: Option<usize>,
    seam: u64,
    normalized_seam: u64,
    raw_token_before_type: Option<TokenType>,
    raw_token_after_type: Option<TokenType>,
    token_before_span: Option<TokenSpan>,
    token_after_span: Option<TokenSpan>,
    token_before_type: Option<TokenType>,
    token_after_type: Option<TokenType>,
}

/// Build one normalized token-resolution snapshot for one blank seam.
fn blank_seam_token_state(
    semantic_tokens: &[TokenSpan],
    trivia: destack_ast::BlankTrivia,
) -> BlankSeamTokenState {
    let (
        raw_token_before,
        raw_token_after,
        normalized_token_before,
        normalized_token_after,
        seam,
        normalized_seam,
    ) = normalized_blank_seam_token_indexes(semantic_tokens, trivia);

    let raw_token_before_type = raw_token_before
        .and_then(|index| semantic_tokens.get(index))
        .map(|token| token.token.ty);
    let raw_token_after_type = raw_token_after
        .and_then(|index| semantic_tokens.get(index))
        .map(|token| token.token.ty);

    let token_before = normalized_token_before.or(raw_token_before);
    let token_after = normalized_token_after.or(raw_token_after);
    let token_before_span = token_before
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let token_after_span = token_after
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let token_before_type = token_before_span.map(|token| token.token.ty);
    let token_after_type = token_after_span.map(|token| token.token.ty);

    BlankSeamTokenState {
        raw_token_after,
        token_before,
        token_after,
        seam,
        normalized_seam,
        raw_token_before_type,
        raw_token_after_type,
        token_before_span,
        token_after_span,
        token_before_type,
        token_after_type,
    }
}

/// One comment-shape snapshot for one blank seam.
struct BlankSeamCommentState {
    seam_has_comment: bool,
    seam_has_line_comment: bool,
    file_start_has_comment: bool,
    blank_before_first_comment: bool,
    blank_before_first_comment_in_after_range: bool,
}

/// Build one comment-shape snapshot for one blank seam.
fn blank_seam_comment_state(
    semantic_tokens: &[TokenSpan],
    seam_index: &FormatterTriviaSeamIndex,
    token_state: &BlankSeamTokenState,
    trivia: destack_ast::BlankTrivia,
) -> BlankSeamCommentState {
    let (_, _, after_range_comment_start) = comment_trivia_summary_in_token_range(
        semantic_tokens,
        token_state.raw_token_after,
        token_state.token_after,
    );

    let raw_boundary_has_trivia = token_state
        .raw_token_before_type
        .is_some_and(token_type_is_trivia)
        || token_state
            .raw_token_after_type
            .is_some_and(token_type_is_trivia);
    let seam_has_comment = seam_index.comment_seams.contains(&token_state.seam)
        || (raw_boundary_has_trivia
            && seam_index
                .comment_seams
                .contains(&token_state.normalized_seam));
    let seam_has_line_comment = seam_index.line_comment_seams.contains(&token_state.seam)
        || (raw_boundary_has_trivia
            && seam_index
                .line_comment_seams
                .contains(&token_state.normalized_seam));
    let file_start_has_comment = seam_has_comment
        || token_state
            .raw_token_after_type
            .is_some_and(token_type_is_comment_trivia);

    let blank_before_first_comment = seam_index
        .first_comment_start_by_seam
        .get(&token_state.seam)
        .copied()
        .or_else(|| {
            seam_index
                .first_comment_start_by_seam
                .get(&token_state.normalized_seam)
                .copied()
        })
        .is_some_and(|start| trivia.span.end <= start);
    let blank_before_first_comment_in_after_range =
        after_range_comment_start.is_some_and(|start| trivia.span.end <= start);

    BlankSeamCommentState {
        seam_has_comment,
        seam_has_line_comment,
        file_start_has_comment,
        blank_before_first_comment,
        blank_before_first_comment_in_after_range,
    }
}

/// One owner-topology snapshot for one blank seam.
struct BlankSeamOwnerState {
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
    preceding_owner_before_semicolon: Option<u32>,
    following_owner_after_comment: Option<u32>,
    shared_expression_owner: Option<u32>,
    preceding_match_case_owner: Option<u32>,
    following_owner_is_argument: bool,
    token_after_comment_continues_chain_or_index: bool,
}

/// Build one owner-topology snapshot for one blank seam.
fn blank_seam_owner_state(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    semantic_tokens: &[TokenSpan],
    owner_index: &FormatterTriviaOwnerIndex,
    token_state: &BlankSeamTokenState,
    token_before_is_semicolon: bool,
) -> BlankSeamOwnerState {
    let preceding_owner = end_owner_at_token(owner_index, token_state.token_before);
    let following_owner = start_owner_at_token(owner_index, token_state.token_after);
    let preceding_token_owner = token_state
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let preceding_owner_before_semicolon = owner_before_semicolon(
        tree,
        parents,
        semantic_tokens,
        token_state.token_before,
        token_before_is_semicolon,
        preceding_token_owner,
        preceding_owner,
    );

    let token_after_comment_next_index = token_state
        .raw_token_after
        .and_then(|token_index| next_non_trivia_token_index(semantic_tokens, token_index));
    let following_owner_after_comment =
        start_owner_at_token(owner_index, token_after_comment_next_index);
    let token_after_comment_target_type = token_after_comment_next_index
        .and_then(|token_index| semantic_tokens.get(token_index))
        .map(|token| token.token.ty);
    let token_after_comment_continues_chain_or_index = matches!(
        token_state.token_after_type,
        Some(TokenType::Dot | TokenType::OpenBracket)
    ) || matches!(
        (token_state.token_after_type, token_after_comment_target_type),
        (Some(comment_type), Some(TokenType::Dot | TokenType::OpenBracket))
            if token_type_is_comment_trivia(comment_type)
    );

    let shared_expression_owner = preceding_owner
        .zip(following_owner)
        .and_then(|(preceding_owner, following_owner)| {
            lowest_common_owner_ancestor(tree, parents, preceding_owner, following_owner)
        })
        .and_then(|owner| (tree.get_node_type(owner) == NodeType::Expression).then_some(owner));
    let preceding_match_case_owner = preceding_owner.and_then(|owner| {
        promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::MatchCase)
    });
    let following_owner_is_argument =
        following_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Argument);

    BlankSeamOwnerState {
        preceding_owner,
        following_owner,
        preceding_token_owner,
        preceding_owner_before_semicolon,
        following_owner_after_comment,
        shared_expression_owner,
        preceding_match_case_owner,
        following_owner_is_argument,
        token_after_comment_continues_chain_or_index,
    }
}

/// One seam facts snapshot used by blank seam attachment phases.
#[derive(Clone, Copy)]
struct BlankSeamFacts {
    token_after: Option<usize>,
    token_after_span: Option<TokenSpan>,
    token_before_type: Option<TokenType>,
    token_after_type: Option<TokenType>,
    seam_has_comment: bool,
    seam_has_line_comment: bool,
    token_before_is_comment: bool,
    token_after_is_comment: bool,
    token_before_is_semicolon: bool,
    token_before_is_open_parenthesis: bool,
    token_before_is_comma: bool,
    token_before_is_colon: bool,
    token_before_is_statement_end: bool,
    token_before_is_top_level_statement_end: bool,
    token_before_is_else: bool,
    token_after_is_else: bool,
    token_after_is_at: bool,
    token_after_is_semicolon: bool,
    token_after_is_close_brace: bool,
    token_after_is_open_parenthesis: bool,
    token_after_is_chain_or_index_boundary: bool,
    semicolon_guard_seam: SemicolonGuardCommentSeam,
    is_if_semicolon_guard_array_separator_seam: bool,
}

/// Build one seam facts snapshot for blank seam phase handlers.
fn blank_seam_facts(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    semantic_tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    token_state: &BlankSeamTokenState,
    comment_state: &BlankSeamCommentState,
    owner_state: &BlankSeamOwnerState,
) -> BlankSeamFacts {
    let token_before_type = token_state.token_before_type;
    let token_after_type = token_state.token_after_type;
    let token_before_span = token_state.token_before_span;
    let token_after_span = token_state.token_after_span;
    let token_after = token_state.token_after;

    let token_before_is_semicolon = token_before_type == Some(TokenType::Semicolon);
    let token_before_is_open_parenthesis = token_before_type == Some(TokenType::OpenParenthesis);
    let token_before_is_comma = token_before_type == Some(TokenType::Comma);
    let token_before_is_colon = token_before_type == Some(TokenType::Colon);
    let token_before_is_statement_end = matches!(
        token_before_type,
        Some(TokenType::Semicolon | TokenType::CloseBrace | TokenType::CloseParenthesis)
    );
    let token_before_is_top_level_statement_end = matches!(
        token_before_type,
        Some(
            TokenType::Semicolon
                | TokenType::CloseBrace
                | TokenType::CloseParenthesis
                | TokenType::CloseBracket
        )
    );
    let token_after_is_at = token_after_type == Some(TokenType::At);
    let token_after_is_semicolon = token_after_type == Some(TokenType::Semicolon);
    let token_after_is_close_brace = token_after_type == Some(TokenType::CloseBrace);
    let token_after_is_open_parenthesis = token_after_type == Some(TokenType::OpenParenthesis);
    let token_after_is_chain_or_index_boundary = matches!(
        token_after_type,
        Some(TokenType::Dot | TokenType::OpenBracket)
    );

    let semicolon_guard_seam = classify_semicolon_guard_comment_seam(
        tree,
        parents,
        semantic_tokens,
        token_before_type,
        token_state.token_before,
        token_after_type,
        token_after,
    );
    let is_if_semicolon_guard_array_separator_seam = owner_has_if_statement_ancestor(
        tree,
        parents,
        owner_state.preceding_owner_before_semicolon,
    ) && semicolon_guard_targets_array_literal(
        semicolon_guard_seam,
        token_before_is_semicolon,
        token_after_type,
    );

    let token_after_keyword = comment_seam_keyword(token_keyword_by_span, token_after_span);
    let token_before_keyword = comment_seam_keyword(token_keyword_by_span, token_before_span);
    let token_after_is_else = token_after_keyword == CommentSeamKeyword::Else;
    let token_before_is_else = token_before_keyword == CommentSeamKeyword::Else;

    let seam_has_comment = comment_state.seam_has_comment;
    let seam_has_line_comment = comment_state.seam_has_line_comment;
    let token_before_is_comment = token_before_type.is_some_and(token_type_is_comment_trivia);
    let token_after_is_comment = token_after_type.is_some_and(token_type_is_comment_trivia);

    BlankSeamFacts {
        token_after,
        token_after_span,
        token_before_type,
        token_after_type,
        seam_has_comment,
        seam_has_line_comment,
        token_before_is_comment,
        token_after_is_comment,
        token_before_is_semicolon,
        token_before_is_open_parenthesis,
        token_before_is_comma,
        token_before_is_colon,
        token_before_is_statement_end,
        token_before_is_top_level_statement_end,
        token_before_is_else,
        token_after_is_else,
        token_after_is_at,
        token_after_is_semicolon,
        token_after_is_close_brace,
        token_after_is_open_parenthesis,
        token_after_is_chain_or_index_boundary,
        semicolon_guard_seam,
        is_if_semicolon_guard_array_separator_seam,
    }
}

/// Run ordered specialized blank seam handlers before fallback ownership.
fn try_attach_blank_specialized_handlers(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    comment_state: &BlankSeamCommentState,
    owner_state: &BlankSeamOwnerState,
    facts: BlankSeamFacts,
) -> Option<(Option<u32>, AnnotationPosition)> {
    if let Some(attachment) = try_attach_comment_shape_blank_seam(
        tree,
        parents,
        comment_state,
        owner_state,
        facts.semicolon_guard_seam,
        facts.token_before_is_statement_end,
        facts.token_before_is_colon,
        facts.token_after_is_else,
        facts.token_before_is_comment,
        facts.token_after_is_comment,
    ) {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_semicolon_guard_blank_seam(
        tree,
        comment_state,
        owner_state,
        facts.semicolon_guard_seam,
        facts.is_if_semicolon_guard_array_separator_seam,
    ) {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_statement_end_comment_blank_seam(
        tree,
        parents,
        comment_state,
        owner_state,
        facts.semicolon_guard_seam,
        facts.token_before_is_statement_end,
    ) {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_chain_delimiter_blank_seam(
        tree,
        comment_state,
        owner_state,
        facts.token_after_span,
        facts.token_before_is_semicolon,
        facts.token_before_is_comma,
        facts.token_before_is_else,
        facts.token_after_is_chain_or_index_boundary,
        facts.token_after_is_semicolon,
        facts.token_after_is_close_brace,
    ) {
        return Some(attachment);
    }

    try_attach_decorator_blank_seam(
        tree,
        parents,
        owner_index,
        owner_state,
        facts.token_after_span,
        facts.token_before_is_semicolon,
        facts.token_after_is_at,
        facts.token_after,
    )
}

/// Run ordered generic blank seam handlers before default owner fallback.
fn try_attach_blank_generic_handlers(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_state: &BlankSeamOwnerState,
    facts: BlankSeamFacts,
) -> Option<(Option<u32>, AnnotationPosition)> {
    if !(facts.token_before_is_semicolon && facts.seam_has_comment)
        && seam_is_top_level_statement_spacing_gap(
            tree,
            parents,
            owner_state.preceding_owner,
            owner_state.following_owner,
            owner_state.preceding_token_owner,
            facts.seam_has_comment,
            facts.seam_has_line_comment,
            facts.token_before_is_top_level_statement_end,
            facts.token_before_is_semicolon,
        )
    {
        return Some(blank_infix_attachment());
    }

    if facts.token_before_type == Some(TokenType::Assign)
        && facts.token_after_type == Some(TokenType::OpenParenthesis)
        && let Some(mut target_node) = owner_state.following_owner
    {
        if tree.get_node_type(target_node) != NodeType::Expression
            && let Some(expression_target) = promote_owner_to_node_type_ancestor(
                tree,
                parents,
                target_node,
                NodeType::Expression,
            )
        {
            target_node = expression_target;
        }

        return Some(block_prefix_attachment(tree, target_node));
    }

    if owner_state.following_owner_is_argument
        && (facts.token_after_is_open_parenthesis || facts.token_before_is_open_parenthesis)
    {
        return Some(blank_infix_attachment());
    }

    None
}

/// Attach one blank seam by default ownership fallback ordering.
fn attach_blank_default_fallback(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    trivia: destack_ast::BlankTrivia,
    owner_state: &BlankSeamOwnerState,
    facts: BlankSeamFacts,
) -> (Option<u32>, AnnotationPosition) {
    if let Some(target_node) = owner_state.following_owner {
        let target_node = facts
            .token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);

        return block_prefix_attachment(tree, target_node);
    }

    if let Some(target_node) = owner_state.preceding_owner {
        return block_postfix_attachment(tree, target_node);
    }

    if let Some(target_node) =
        find_smallest_owner_enclosing_range(tree, trivia.span.start, trivia.span.end)
    {
        return block_prefix_attachment(tree, target_node);
    }

    blank_infix_attachment()
}

/// Attach one if-specific or generic semicolon-guard blank seam.
fn try_attach_semicolon_guard_blank_seam(
    tree: &NodeTree,
    comment_state: &BlankSeamCommentState,
    owner_state: &BlankSeamOwnerState,
    semicolon_guard_seam: SemicolonGuardCommentSeam,
    is_if_semicolon_guard_array_separator_seam: bool,
) -> Option<(Option<u32>, AnnotationPosition)> {
    let seam_has_comment = comment_state.seam_has_comment;
    let blank_before_first_comment = comment_state.blank_before_first_comment;
    let preceding_owner_before_semicolon = owner_state.preceding_owner_before_semicolon;
    let preceding_owner = owner_state.preceding_owner;
    let following_owner = owner_state.following_owner;

    // if semicolon-guard array seams
    if seam_has_comment
        && blank_before_first_comment
        && is_if_semicolon_guard_array_separator_seam
        && let Some(target_node) = following_owner
            .or(preceding_owner_before_semicolon)
            .or(preceding_owner)
    {
        return Some(block_prefix_attachment(tree, target_node));
    }

    // generic semicolon guard seams
    if seam_has_comment && semicolon_guard_seam.has_guard_shape() {
        return Some(blank_infix_attachment());
    }

    None
}

/// Attach one statement-end blank seam with comment-owned ownership.
fn try_attach_statement_end_comment_blank_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    comment_state: &BlankSeamCommentState,
    owner_state: &BlankSeamOwnerState,
    semicolon_guard_seam: SemicolonGuardCommentSeam,
    token_before_is_statement_end: bool,
) -> Option<(Option<u32>, AnnotationPosition)> {
    let seam_has_comment = comment_state.seam_has_comment;
    let seam_has_line_comment = comment_state.seam_has_line_comment;
    let blank_before_first_comment = comment_state.blank_before_first_comment;

    let preceding_token_owner = owner_state.preceding_token_owner;
    let preceding_owner = owner_state.preceding_owner;
    let preceding_owner_before_semicolon = owner_state.preceding_owner_before_semicolon;
    let following_owner = owner_state.following_owner;
    let following_owner_after_comment = owner_state.following_owner_after_comment;
    let token_after_comment_continues_chain_or_index =
        owner_state.token_after_comment_continues_chain_or_index;

    // statement-end seams with line comments
    if seam_has_line_comment && blank_before_first_comment && token_before_is_statement_end {
        if token_after_comment_continues_chain_or_index
            && let Some(target_node) = preceding_token_owner
                .or(preceding_owner_before_semicolon)
                .or(preceding_owner)
        {
            return Some(block_postfix_attachment(tree, target_node));
        }

        if let Some(target_node) = following_owner.or(following_owner_after_comment) {
            let target_node =
                promote_owner_to_nearest_statement_boundary(tree, parents, target_node);
            return Some(block_prefix_attachment(tree, target_node));
        }

        if let Some(target_node) = preceding_owner_before_semicolon
            .or(preceding_token_owner)
            .or(preceding_owner)
        {
            return Some(block_postfix_attachment(tree, target_node));
        }
    }

    // statement-end seams with non-line comments
    if seam_has_comment
        && !seam_has_line_comment
        && blank_before_first_comment
        && token_before_is_statement_end
        && !semicolon_guard_seam.has_guard_shape()
        && let Some(target_node) = preceding_owner_before_semicolon
            .or(preceding_token_owner)
            .or(preceding_owner)
    {
        return Some(block_postfix_attachment(tree, target_node));
    }

    None
}

/// Attach comment-shape blank seam rules before semicolon and statement routing.
fn try_attach_comment_shape_blank_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    comment_state: &BlankSeamCommentState,
    owner_state: &BlankSeamOwnerState,
    semicolon_guard_seam: SemicolonGuardCommentSeam,
    token_before_is_statement_end: bool,
    token_before_is_colon: bool,
    token_after_is_else: bool,
    token_before_is_comment: bool,
    token_after_is_comment: bool,
) -> Option<(Option<u32>, AnnotationPosition)> {
    let seam_has_comment = comment_state.seam_has_comment;
    let blank_before_first_comment = comment_state.blank_before_first_comment;
    let blank_before_first_comment_in_after_range =
        comment_state.blank_before_first_comment_in_after_range;

    let preceding_token_owner = owner_state.preceding_token_owner;
    let preceding_owner = owner_state.preceding_owner;
    let preceding_owner_before_semicolon = owner_state.preceding_owner_before_semicolon;
    let following_owner = owner_state.following_owner;
    let shared_expression_owner = owner_state.shared_expression_owner;
    let preceding_match_case_owner = owner_state.preceding_match_case_owner;
    let token_after_comment_continues_chain_or_index =
        owner_state.token_after_comment_continues_chain_or_index;

    // preserve blank seams between adjacent comments
    if seam_has_comment
        && blank_before_first_comment
        && token_before_is_comment
        && token_after_is_comment
        && let Some(target_node) = following_owner.or(preceding_owner)
    {
        return Some(block_prefix_attachment(tree, target_node));
    }

    // statement-end chain seams with comments
    if (blank_before_first_comment || blank_before_first_comment_in_after_range)
        && token_before_is_statement_end
        && token_after_comment_continues_chain_or_index
        && !semicolon_guard_seam.has_guard_shape()
        && let Some(target_node) = preceding_token_owner
            .or(preceding_owner)
            .or(preceding_owner_before_semicolon)
    {
        return Some(block_postfix_attachment(tree, target_node));
    }

    // switch-label seams with comments before case labels
    if token_before_is_colon
        && seam_has_comment
        && blank_before_first_comment
        && let Some(target_node) = preceding_match_case_owner
    {
        return Some(block_postfix_attachment(tree, target_node));
    }

    // else seams with comment-owned blank trivia
    if seam_has_comment && token_after_is_else {
        if blank_before_first_comment && let Some(target_node) = shared_expression_owner {
            return Some(block_postfix_attachment(tree, target_node));
        }

        if blank_before_first_comment && let Some(mut target_node) = following_owner {
            if tree.get_node_type(target_node) != NodeType::Expression
                && let Some(expression_target) = promote_owner_to_node_type_ancestor(
                    tree,
                    parents,
                    target_node,
                    NodeType::Expression,
                )
            {
                target_node = expression_target;
            }

            return Some(block_prefix_attachment(tree, target_node));
        }

        return Some(blank_infix_attachment());
    }

    None
}

/// Attach chain, delimiter, and suppression blank seam rules.
fn try_attach_chain_delimiter_blank_seam(
    tree: &NodeTree,
    comment_state: &BlankSeamCommentState,
    owner_state: &BlankSeamOwnerState,
    token_after_span: Option<TokenSpan>,
    token_before_is_semicolon: bool,
    token_before_is_comma: bool,
    token_before_is_else: bool,
    token_after_is_chain_or_index_boundary: bool,
    token_after_is_semicolon: bool,
    token_after_is_close_brace: bool,
) -> Option<(Option<u32>, AnnotationPosition)> {
    let seam_has_comment = comment_state.seam_has_comment;
    let blank_before_first_comment = comment_state.blank_before_first_comment;
    let blank_before_first_comment_in_after_range =
        comment_state.blank_before_first_comment_in_after_range;

    let preceding_owner = owner_state.preceding_owner;
    let following_owner = owner_state.following_owner;

    // chain or index seams with comments
    if seam_has_comment && token_after_is_chain_or_index_boundary {
        if blank_before_first_comment && let Some(target_node) = preceding_owner {
            return Some(block_postfix_attachment(tree, target_node));
        }

        if let Some(token) = token_after_span
            && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token.span)
        {
            return Some(block_prefix_attachment(tree, target_node));
        }
    }

    // semicolon-own-line comment seams
    if seam_has_comment
        && blank_before_first_comment
        && token_before_is_semicolon
        && token_after_is_semicolon
    {
        return Some(blank_infix_attachment());
    }

    // comma seams with comments
    if seam_has_comment
        && (blank_before_first_comment || blank_before_first_comment_in_after_range)
        && token_before_is_comma
        && let Some(target_node) = following_owner
    {
        return Some(block_prefix_attachment(tree, target_node));
    }

    // semicolon and close-brace seam suppression
    if token_after_is_semicolon || token_after_is_close_brace {
        return Some(blank_infix_attachment());
    }

    // else-to-body seam suppression
    if token_before_is_else {
        return Some(blank_infix_attachment());
    }

    None
}

/// Attach decorator-adjacent blank seam rules near `@`.
fn try_attach_decorator_blank_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &FormatterTriviaOwnerIndex,
    owner_state: &BlankSeamOwnerState,
    token_after_span: Option<TokenSpan>,
    token_before_is_semicolon: bool,
    token_after_is_at: bool,
    token_after: Option<usize>,
) -> Option<(Option<u32>, AnnotationPosition)> {
    let preceding_owner = owner_state.preceding_owner;
    let following_owner = owner_state.following_owner;

    if !token_after_is_at {
        return None;
    }

    if token_before_is_semicolon
        && let Some(token_after_index) = token_after
        && let Some(target_node) = find_owner_at_or_after_token_with_node_type(
            tree,
            owner_index,
            token_after_index,
            NodeType::Member,
        )
        .and_then(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
        })
        && let Some(preceding_declaration) = preceding_owner
            .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        && let Some(target_declaration) =
            promote_owner_to_declaration_ancestor(tree, parents, target_node)
        && preceding_declaration == target_declaration
    {
        return Some(block_prefix_attachment(tree, target_node));
    }

    if token_before_is_semicolon
        && let Some(target_node) = preceding_owner
        && tree.get_node_type(target_node) == NodeType::Member
    {
        return Some(block_postfix_attachment(tree, target_node));
    }

    if let Some(token) = token_after_span
        && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token.span)
        && tree.get_node_type(target_node) == NodeType::Member
    {
        return Some(block_prefix_attachment(tree, target_node));
    }

    if let Some(target_node) = following_owner
        && tree.get_node_type(target_node) == NodeType::Member
    {
        return Some(block_prefix_attachment(tree, target_node));
    }

    let declaration_target = token_after
        .and_then(|token_after_index| {
            find_owner_at_or_after_token_with_node_type(
                tree,
                owner_index,
                token_after_index,
                NodeType::Declaration,
            )
        })
        .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        .or_else(|| {
            following_owner
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
                .or(following_owner)
        });

    if let Some(target_node) = declaration_target {
        return Some(block_prefix_attachment(tree, target_node));
    }

    None
}

pub(crate) fn blank_trivia_attachment(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    trivia: destack_ast::BlankTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    seam_index: &FormatterTriviaSeamIndex,
    parents: &NodeParentIndex,
) -> (Option<u32>, AnnotationPosition) {
    // seam token resolution
    let token_state = blank_seam_token_state(semantic_tokens, trivia);
    let token_before = token_state.token_before;
    let token_after = token_state.token_after;

    // seam comment shape
    let comment_state = blank_seam_comment_state(semantic_tokens, seam_index, &token_state, trivia);

    // file start
    if token_before.is_none() {
        let following_owner = start_owner_at_token(owner_index, token_after);
        if comment_state.file_start_has_comment
            && let Some(mut target_node) = following_owner
        {
            if tree.get_node_type(target_node) != NodeType::Expression
                && let Some(expression_target) = promote_owner_to_node_type_ancestor(
                    tree,
                    parents,
                    target_node,
                    NodeType::Expression,
                )
            {
                target_node = expression_target;
            }

            return block_prefix_attachment(tree, target_node);
        }

        return blank_infix_attachment();
    }

    // file-end blank seams are plain spacing: do not attach them to the last owner
    if token_after.is_none() {
        return blank_infix_attachment();
    }

    // token shape
    if token_state.token_after_type == Some(TokenType::End) {
        return blank_infix_attachment();
    }

    // owner topology
    let owner_state = blank_seam_owner_state(
        tree,
        parents,
        semantic_tokens,
        owner_index,
        &token_state,
        token_state.token_before_type == Some(TokenType::Semicolon),
    );

    // seam facts
    let facts = blank_seam_facts(
        tree,
        parents,
        semantic_tokens,
        token_keyword_by_span,
        &token_state,
        &comment_state,
        &owner_state,
    );

    // specialized seam handlers
    if let Some(attachment) = try_attach_blank_specialized_handlers(
        tree,
        parents,
        owner_index,
        &comment_state,
        &owner_state,
        facts,
    ) {
        return attachment;
    }

    // generic seam handlers
    if let Some(attachment) = try_attach_blank_generic_handlers(tree, parents, &owner_state, facts)
    {
        return attachment;
    }

    // default ownership fallback
    attach_blank_default_fallback(tree, parents, trivia, &owner_state, facts)
}
