use ast::{
    AnnotationPosition, CommentDirective, Expression, Keyword, LocalNodeId, NodeParentIndex,
    NodeTree, NodeType, TokenSpan, TokenType,
};
use destack_ast as ast;
use destack_source::{File, Span};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use super::facts::{
    next_non_trivia_token_index, previous_non_trivia_token_index, token_type_is_comment_trivia,
    token_type_is_trivia, token_type_is_whitespace_trivia,
};
use super::ownership::{
    find_owner_at_or_after_token, find_owner_at_or_after_token_with_node_type,
    find_preferred_owner_starting_at, find_smallest_owner_enclosing_range,
    find_smallest_owner_enclosing_token, is_trivia_excluded_owner_node_id,
    normalize_owner_with_shared_end, normalize_trivia_target_owner, promote_owner_by_shared_start,
    promote_owner_to_declaration_ancestor, promote_owner_to_node_type_ancestor,
    promote_rhs_expression_owner,
};
use super::semantic::{
    project_parser_semantic_annotations, push_annotation_projection_entry,
    sort_annotation_projection_by_source,
};
use crate::{Annotation, AnnotationEntry};

const NO_TOKEN_INDEX: u32 = u32::MAX;

/// Return rhs owner for one doc annotation that belongs to an assignment seam.
pub(super) fn assignment_like_rhs_owner_for_doc_annotation(
    file: &File,
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(u32, AnnotationPosition)> {
    let annotation_token_index =
        tokens.partition_point(|token| token.span.start < annotation_span.start);
    if annotation_token_index >= tokens.len() {
        return None;
    }

    let previous_index = previous_non_trivia_token_index(tokens, annotation_token_index)?;
    let previous_token_type = tokens[previous_index].token.ty;
    if previous_token_type != TokenType::Assign {
        return None;
    }

    // assignment token span anchors rhs ownership and line-prefix decisions
    let previous_token_span = tokens[previous_index].span;

    // next semantic token after annotation drives rhs fallback ownership
    let mut token_after_annotation_index =
        tokens.partition_point(|token| token.span.start < annotation_span.end);
    let mut token_after_span = None;
    let mut has_intervening_comment_trivia = false;
    while token_after_annotation_index < tokens.len() {
        let token = tokens[token_after_annotation_index];
        let token_type = token.token.ty;

        if token_type_is_whitespace_trivia(token_type) {
            token_after_annotation_index += 1;
            continue;
        }

        if token_type_is_comment_trivia(token_type) {
            has_intervening_comment_trivia = true;
            token_after_annotation_index += 1;
            continue;
        }

        token_after_span = Some(token.span);
        break;
    }
    let token_after_span = token_after_span?;
    if has_intervening_comment_trivia || annotation_span.start >= token_after_span.start {
        return None;
    }

    // primary rhs owner comes from the assignment seam
    let assignment_owner =
        find_smallest_owner_enclosing_token(tree, previous_token_span).and_then(|owner_id| {
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
                promote_owner_to_declaration_ancestor(tree, parents, owner_id)
            }?;
            let declaration_id = LocalNodeId::<ast::Declaration>::new(declaration_owner);
            match tree.get(declaration_id) {
                ast::Declaration::Type { value, .. } => Some(value.id),
                _ => None,
            }
        });

    // fallback rhs owner starts at the first expression after annotation
    let fallback_expression_owner = find_preferred_owner_starting_at(tree, token_after_span)
        .and_then(|owner_id| {
            if tree.get_node_type(owner_id) == NodeType::Expression {
                Some(owner_id)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
            }
        })
        .or_else(|| {
            find_owner_at_or_after_token_with_node_type(
                tree,
                owner_index,
                token_after_annotation_index,
                NodeType::Expression,
            )
        });

    let expression_owner = assignment_owner.or(fallback_expression_owner)?;
    let expression_owner =
        promote_rhs_expression_owner(tree, parents, expression_owner, Some(token_after_span));

    // inline assignment seam doc comments stay line-prefix
    let annotation_starts_on_anchor_line =
        file.is_same_line(previous_token_span.start, annotation_span.start);
    let annotation_is_multiline = annotation_span.start < annotation_span.end
        && !file.is_same_line(annotation_span.start, annotation_span.end.saturating_sub(1));
    let position = if annotation_starts_on_anchor_line && !annotation_is_multiline {
        AnnotationPosition::LinePrefix
    } else {
        AnnotationPosition::BlockPrefix
    };

    Some((expression_owner, position))
}

/// Return trailing statement owner for one doc annotation after terminal semicolon.
pub(super) fn trailing_statement_owner_for_doc_annotation(
    file: &File,
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(u32, AnnotationPosition)> {
    let annotation_token_index =
        tokens.partition_point(|token| token.span.start < annotation_span.start);
    if annotation_token_index >= tokens.len() {
        return None;
    }

    let previous_index = previous_non_trivia_token_index(tokens, annotation_token_index)?;
    if tokens[previous_index].token.ty != TokenType::Semicolon {
        return None;
    }

    let token_after_annotation_index =
        tokens.partition_point(|token| token.span.start < annotation_span.end);
    let has_following_owner =
        find_owner_at_or_after_token(tree, tokens, token_after_annotation_index).is_some()
            || find_owner_at_or_after_token_with_node_type(
                tree,
                owner_index,
                token_after_annotation_index,
                NodeType::Declaration,
            )
            .is_some();
    if has_following_owner {
        return None;
    }

    let semicolon_span = tokens[previous_index].span;
    let owner = find_smallest_owner_enclosing_token(tree, semicolon_span)?;
    let owner = normalize_owner_with_shared_end(tree, parents, owner, Some(semicolon_span));
    let is_same_line =
        file.is_same_line(semicolon_span.end.saturating_sub(1), annotation_span.start);
    let position = if is_same_line {
        AnnotationPosition::LinePostfixBoundary
    } else {
        AnnotationPosition::BlockPostfix
    };

    Some((owner, position))
}

/// Return rhs owner for one doc annotation that follows one opening parenthesis seam.
pub(super) fn parenthesized_rhs_owner_for_doc_annotation(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(u32, AnnotationPosition)> {
    let annotation_token_index =
        tokens.partition_point(|token| token.span.start < annotation_span.start);
    if annotation_token_index >= tokens.len() {
        return None;
    }

    let previous_index = previous_non_trivia_token_index(tokens, annotation_token_index)?;
    if tokens[previous_index].token.ty != TokenType::OpenParenthesis {
        return None;
    }
    let previous_token_span = tokens[previous_index].span;

    let mut token_after_annotation_index =
        tokens.partition_point(|token| token.span.start < annotation_span.end);
    let mut token_after_span = None;
    let mut has_intervening_comment_trivia = false;
    while token_after_annotation_index < tokens.len() {
        let token = tokens[token_after_annotation_index];
        let token_type = token.token.ty;

        if token_type_is_whitespace_trivia(token_type) {
            token_after_annotation_index += 1;
            continue;
        }

        if token_type_is_comment_trivia(token_type) {
            has_intervening_comment_trivia = true;
            token_after_annotation_index += 1;
            continue;
        }

        token_after_span = Some(token.span);
        break;
    }
    let token_after_span = token_after_span?;
    if has_intervening_comment_trivia || annotation_span.start >= token_after_span.start {
        return None;
    }

    let expression_owner = find_smallest_owner_enclosing_token(tree, token_after_span)
        .or_else(|| find_owner_at_or_after_token(tree, tokens, token_after_annotation_index))
        .filter(|owner_id| tree.get_node_type(*owner_id) == NodeType::Expression)?;
    let annotation_starts_on_anchor_line =
        file.is_same_line(previous_token_span.start, annotation_span.start);
    let annotation_is_multiline = annotation_span.start < annotation_span.end
        && !file.is_same_line(annotation_span.start, annotation_span.end.saturating_sub(1));
    let position = if annotation_starts_on_anchor_line && !annotation_is_multiline {
        AnnotationPosition::LinePrefix
    } else {
        AnnotationPosition::BlockPrefix
    };

    Some((expression_owner, position))
}

#[derive(Debug)]
pub(crate) struct TriviaOwnerIndex {
    pub(crate) owner_start_by_token: Vec<Option<u32>>,
    pub(crate) owner_end_by_token: Vec<Option<u32>>,
    pub(crate) nearest_owner_start_by_token: Vec<Option<u32>>,
    pub(crate) nearest_owner_end_by_token: Vec<Option<u32>>,
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

/// Return whether one token kind is an opening delimiter.
#[inline]
fn is_open_delimiter_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
    )
}

/// Return whether one token kind is a closing delimiter.
#[inline]
fn is_close_delimiter_token(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::CloseParenthesis | TokenType::CloseBrace | TokenType::CloseBracket
    )
}

/// Return whether one open and close delimiter token pair matches.
#[inline]
fn delimiters_match(open: TokenType, close: TokenType) -> bool {
    matches!(
        (open, close),
        (TokenType::OpenParenthesis, TokenType::CloseParenthesis)
            | (TokenType::OpenBrace, TokenType::CloseBrace)
            | (TokenType::OpenBracket, TokenType::CloseBracket)
    )
}

/// Resolve one seam owner lazily from seam token range.
fn comment_enclosing_owner(
    tree: &NodeTree,
    token_before_span: Option<TokenSpan>,
    token_after_span: Option<TokenSpan>,
    cached_owner: &mut Option<Option<u32>>,
) -> Option<u32> {
    if let Some(owner) = *cached_owner {
        return owner;
    }

    let owner = token_before_span
        .zip(token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        })
        .or_else(|| {
            token_before_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        })
        .or_else(|| {
            token_after_span.and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        });

    *cached_owner = Some(owner);
    owner
}

/// Build owner indexes for formatter-side trivia attachment.
pub(crate) fn trivia_owner_index(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
) -> TriviaOwnerIndex {
    let mut start_token_by_offset = FxHashMap::<u32, usize>::default();
    start_token_by_offset.reserve(semantic_tokens.len());
    let mut end_token_by_offset = FxHashMap::<u32, usize>::default();
    end_token_by_offset.reserve(semantic_tokens.len());

    // attachable token maps
    for (index, token) in semantic_tokens.iter().copied().enumerate() {
        if matches!(token.token.ty, TokenType::Newline | TokenType::End) {
            continue;
        }

        start_token_by_offset.insert(token.span.start, index);
        end_token_by_offset.insert(token.span.end, index);
    }

    let mut owner_start_by_token = vec![None; semantic_tokens.len()];
    let mut owner_end_by_token = vec![None; semantic_tokens.len()];
    let mut owner_start_length_by_token = vec![u32::MAX; semantic_tokens.len()];
    let mut owner_end_length_by_token = vec![u32::MAX; semantic_tokens.len()];
    let mut owner_start_kind_rank_by_token = vec![u8::MAX; semantic_tokens.len()];
    let mut owner_end_kind_rank_by_token = vec![u8::MAX; semantic_tokens.len()];

    // choose smallest owner for one token seam and share one node scan for start and end
    let mut node_id = 0u32;
    while node_id < tree.next_id() {
        if is_trivia_excluded_owner_node_id(tree, node_id) {
            node_id += 1;
            continue;
        }

        let span = tree.get_span_by_id(node_id);
        let length = span.end.saturating_sub(span.start);
        let node_kind_rank = if tree.get_node_type(node_id) == NodeType::Expression {
            1
        } else {
            0
        };

        if let Some(token_index) = start_token_by_offset.get(&span.start).copied() {
            let best_length = owner_start_length_by_token[token_index];
            let best_kind_rank = owner_start_kind_rank_by_token[token_index];
            let best_owner = owner_start_by_token[token_index];
            let should_replace = length < best_length
                || (length == best_length && node_kind_rank < best_kind_rank)
                || (length == best_length
                    && node_kind_rank == best_kind_rank
                    && best_owner.is_none_or(|best| node_id < best));
            if should_replace {
                owner_start_length_by_token[token_index] = length;
                owner_start_kind_rank_by_token[token_index] = node_kind_rank;
                owner_start_by_token[token_index] = Some(node_id);
            }
        }

        if let Some(token_index) = end_token_by_offset.get(&span.end).copied() {
            let best_length = owner_end_length_by_token[token_index];
            let best_kind_rank = owner_end_kind_rank_by_token[token_index];
            let best_owner = owner_end_by_token[token_index];
            let should_replace = length < best_length
                || (length == best_length && node_kind_rank < best_kind_rank)
                || (length == best_length
                    && node_kind_rank == best_kind_rank
                    && best_owner.is_none_or(|best| node_id < best));
            if should_replace {
                owner_end_length_by_token[token_index] = length;
                owner_end_kind_rank_by_token[token_index] = node_kind_rank;
                owner_end_by_token[token_index] = Some(node_id);
            }
        }

        node_id += 1;
    }

    let mut nearest_owner_start_by_token = vec![None; semantic_tokens.len()];
    let mut nearest_owner_start = None;
    for index in (0..semantic_tokens.len()).rev() {
        let token = semantic_tokens[index];
        if matches!(token.token.ty, TokenType::Newline | TokenType::End) {
            nearest_owner_start_by_token[index] = nearest_owner_start;
            continue;
        }

        nearest_owner_start = owner_start_by_token[index].or(nearest_owner_start);
        nearest_owner_start_by_token[index] = nearest_owner_start;
    }

    let mut nearest_owner_end_by_token = vec![None; semantic_tokens.len()];
    let mut nearest_owner_end = None;
    for index in 0..semantic_tokens.len() {
        let token = semantic_tokens[index];
        if matches!(token.token.ty, TokenType::Newline | TokenType::End) {
            nearest_owner_end_by_token[index] = nearest_owner_end;
            continue;
        }

        nearest_owner_end = owner_end_by_token[index].or(nearest_owner_end);
        nearest_owner_end_by_token[index] = nearest_owner_end;
    }

    TriviaOwnerIndex {
        owner_start_by_token,
        owner_end_by_token,
        nearest_owner_start_by_token,
        nearest_owner_end_by_token,
    }
}

/// Resolve one blank trivia attachment to one structural owner and capture position.
pub(crate) fn blank_trivia_attachment(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    trivia: destack_ast::BlankTrivia,
    owner_index: &TriviaOwnerIndex,
    parents: &NodeParentIndex,
) -> (Option<u32>, AnnotationPosition) {
    // seam token resolution
    let raw_token_before = (trivia.boundary.token_before != NO_TOKEN_INDEX)
        .then_some(trivia.boundary.token_before as usize);
    let raw_token_after = (trivia.boundary.token_after != NO_TOKEN_INDEX)
        .then_some(trivia.boundary.token_after as usize);
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

    // file start
    if token_before.is_none() {
        // leading blank seams belong to the first owner when one exists
        if let Some(target_node) = token_after
            .and_then(|index| {
                owner_index
                    .owner_start_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
            .or_else(|| {
                token_after.and_then(|index| {
                    owner_index
                        .nearest_owner_start_by_token
                        .get(index)
                        .and_then(|owner| *owner)
                })
            })
        {
            let target_node = normalize_blank_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        return (None, AnnotationPosition::BlockInfix);
    }

    // file-end blank seams are plain spacing: do not attach them to the last owner
    if token_after.is_none() {
        return (None, AnnotationPosition::BlockInfix);
    }

    // token shape
    if token_after_type == Some(TokenType::End) {
        return (None, AnnotationPosition::BlockInfix);
    }

    // owner topology
    let preceding_owner = token_before
        .and_then(|index| {
            owner_index
                .owner_end_by_token
                .get(index)
                .and_then(|owner| *owner)
        })
        .or_else(|| {
            token_before.and_then(|index| {
                owner_index
                    .nearest_owner_end_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
        });
    let following_owner = token_after
        .and_then(|index| {
            owner_index
                .owner_start_by_token
                .get(index)
                .and_then(|owner| *owner)
        })
        .or_else(|| {
            token_after.and_then(|index| {
                owner_index
                    .nearest_owner_start_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
        });
    let following_owner_is_argument =
        following_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Argument);

    let token_before_is_open_parenthesis = token_before_type == Some(TokenType::OpenParenthesis);
    let token_before_is_comma = token_before_type == Some(TokenType::Comma);
    let token_after_is_semicolon = token_after_type == Some(TokenType::Semicolon);
    let token_after_is_close_brace = token_after_type == Some(TokenType::CloseBrace);
    let token_after_is_close_parenthesis = token_after_type == Some(TokenType::CloseParenthesis);
    let token_after_is_open_parenthesis = token_after_type == Some(TokenType::OpenParenthesis);
    let token_after_is_chain_or_index_boundary = matches!(
        token_after_type,
        Some(TokenType::Dot | TokenType::OpenBracket)
    );

    // chain or index seams without comments are spacing-only markers
    if token_after_is_chain_or_index_boundary {
        return (None, AnnotationPosition::BlockInfix);
    }

    if token_after_is_semicolon || token_after_is_close_brace {
        return (None, AnnotationPosition::BlockInfix);
    }

    if matches!(
        token_after_type,
        Some(
            TokenType::CloseParenthesis
                | TokenType::CloseBracket
                | TokenType::CloseBrace
                | TokenType::Semicolon
        )
    ) {
        return (None, AnnotationPosition::BlockInfix);
    }

    if token_before_type == Some(TokenType::Assign)
        && token_after_type == Some(TokenType::OpenParenthesis)
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

        let target_node = normalize_blank_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    if following_owner_is_argument
        && (token_after_is_open_parenthesis || token_before_is_open_parenthesis)
    {
        return (None, AnnotationPosition::BlockInfix);
    }

    // trailing comma gaps stay with the preceding argument
    if token_before_is_comma
        && token_after_is_close_parenthesis
        && let Some(target_node) = preceding_owner
    {
        let target_node = normalize_blank_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // default ownership fallback
    if let Some(target_node) = following_owner {
        let target_node = token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        let target_node = normalize_blank_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    if let Some(target_node) = preceding_owner {
        let target_node = normalize_blank_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    if let Some(target_node) =
        find_smallest_owner_enclosing_range(tree, trivia.span.start, trivia.span.end)
    {
        let target_node = normalize_blank_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    (None, AnnotationPosition::BlockInfix)
}

/// Resolve one comment trivia target owner and position from one token seam.
pub(crate) fn comment_trivia_attachment(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    _token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    trivia: ast::CommentTrivia,
    owner_index: &TriviaOwnerIndex,
    parents: &NodeParentIndex,
) -> (Option<u32>, AnnotationPosition) {
    let token_before = (trivia.boundary.token_before != NO_TOKEN_INDEX)
        .then_some(trivia.boundary.token_before as usize)
        .and_then(|token_index| {
            let token_type = semantic_tokens.get(token_index)?.token.ty;
            if token_type_is_trivia(token_type) {
                previous_non_trivia_token_index(semantic_tokens, token_index)
            } else {
                Some(token_index)
            }
        })
        .or_else(|| {
            let token_index =
                semantic_tokens.partition_point(|token| token.span.start < trivia.span.start);
            previous_non_trivia_token_index(semantic_tokens, token_index)
        });
    let token_after = (trivia.boundary.token_after != NO_TOKEN_INDEX)
        .then_some(trivia.boundary.token_after as usize)
        .and_then(|token_index| {
            let token_type = semantic_tokens.get(token_index)?.token.ty;
            if token_type_is_trivia(token_type) {
                next_non_trivia_token_index(semantic_tokens, token_index)
            } else {
                Some(token_index)
            }
        })
        .or_else(|| {
            let mut token_index =
                semantic_tokens.partition_point(|token| token.span.start < trivia.span.end);
            while token_index < semantic_tokens.len() {
                let token_type = semantic_tokens[token_index].token.ty;
                if !token_type_is_trivia(token_type) {
                    return Some(token_index);
                }

                token_index += 1;
            }

            None
        });
    let token_before_span = token_before
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let token_after_span = token_after
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let mut following_owner = token_after.and_then(|index| {
        owner_index
            .owner_start_by_token
            .get(index)
            .and_then(|owner| *owner)
            .or_else(|| {
                owner_index
                    .nearest_owner_start_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
    });
    let mut preceding_owner = token_before.and_then(|index| {
        owner_index
            .owner_end_by_token
            .get(index)
            .and_then(|owner| *owner)
            .or_else(|| {
                owner_index
                    .nearest_owner_end_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
    });

    // token spans close seam ownership gaps when index tables do not resolve
    if following_owner.is_none()
        && let Some(token_after_span) = token_after_span
    {
        following_owner = find_preferred_owner_starting_at(tree, token_after_span.span)
            .or_else(|| find_smallest_owner_enclosing_token(tree, token_after_span.span));
    }

    // token spans close seam ownership gaps when index tables do not resolve
    if preceding_owner.is_none()
        && let Some(token_before_span) = token_before_span
    {
        preceding_owner = find_smallest_owner_enclosing_token(tree, token_before_span.span);
    }

    let has_leading_newline = trivia.boundary.newlines.has_leading_newline();
    let has_trailing_newline = trivia.boundary.newlines.has_trailing_newline();
    let comment_is_line = tree.get(trivia.comment).style == ast::CommentStyle::Slash;
    let token_before_type = token_before_span.map(|token| token.token.ty);
    let token_after_type = token_after_span.map(|token| token.token.ty);
    let mut enclosing_owner_cache = None;

    if token_before_type == Some(TokenType::OpenParenthesis)
        && token_after_type == Some(TokenType::CloseParenthesis)
        && let Some(enclosing_owner) = comment_enclosing_owner(
            tree,
            token_before_span,
            token_after_span,
            &mut enclosing_owner_cache,
        )
    {
        let enclosing_owner = normalize_trivia_target_owner(tree, enclosing_owner);
        return (Some(enclosing_owner), AnnotationPosition::BlockInfix);
    }

    if let (Some(token_before_type), Some(token_after_type)) = (token_before_type, token_after_type)
        && token_before_type != TokenType::OpenParenthesis
        && is_open_delimiter_token(token_before_type)
        && is_close_delimiter_token(token_after_type)
        && delimiters_match(token_before_type, token_after_type)
        && let Some(enclosing_owner) = comment_enclosing_owner(
            tree,
            token_before_span,
            token_after_span,
            &mut enclosing_owner_cache,
        )
    {
        let enclosing_owner = normalize_trivia_target_owner(tree, enclosing_owner);
        return (Some(enclosing_owner), AnnotationPosition::BlockInfix);
    }

    attach_comment_default(
        tree,
        semantic_tokens,
        trivia,
        parents,
        token_before,
        token_after,
        token_before_span,
        token_after_span,
        has_leading_newline,
        has_trailing_newline,
        comment_is_line,
        &mut enclosing_owner_cache,
        preceding_owner,
        following_owner,
    )
}

/// Build formatter annotation projection for semantic annotations and trivia.
pub(crate) fn annotation_projection(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
) -> (
    Vec<AnnotationEntry>,
    Vec<SmallVec<[LocalNodeId<Annotation>; 4]>>,
) {
    let node_count = tree.next_id() as usize;
    let mut entries = Vec::new();
    let mut by_node_id = vec![SmallVec::new(); node_count];
    let owner_index = trivia_owner_index(tree, tokens);

    // add parser semantic annotations first
    project_parser_semantic_annotations(
        file,
        tree,
        tokens,
        parents,
        &owner_index,
        &mut entries,
        &mut by_node_id,
    );

    // comment trivia
    let mut comment_targets = Vec::<(Span, u32)>::new();
    for trivia in tree.comment_trivia().iter().copied() {
        let (target_id, position) = comment_trivia_attachment(
            tree,
            tokens,
            token_keyword_by_span,
            trivia,
            &owner_index,
            parents,
        );

        let Some(target_id) = target_id else {
            continue;
        };
        if !push_annotation_projection_entry(
            &mut entries,
            &mut by_node_id,
            target_id,
            Annotation::Comment {
                node: trivia.comment,
                position,
            },
            trivia.span,
        ) {
            continue;
        }

        comment_targets.push((trivia.span, target_id));
    }

    // blank trivia
    for trivia in tree.blank_trivia().iter().copied() {
        let token_before_index = (trivia.boundary.token_before != NO_TOKEN_INDEX)
            .then_some(trivia.boundary.token_before as usize);
        let token_after_index = (trivia.boundary.token_after != NO_TOKEN_INDEX)
            .then_some(trivia.boundary.token_after as usize);
        let token_before_type = token_before_index
            .and_then(|index| tokens.get(index))
            .map(|token| token.token.ty);
        let token_after_type = token_after_index
            .and_then(|index| tokens.get(index))
            .map(|token| token.token.ty);
        let token_before_is_statement_end = matches!(
            token_before_type,
            Some(TokenType::Semicolon | TokenType::CloseBrace | TokenType::CloseParenthesis)
        );
        let token_before_is_if_without_else = token_before_index
            .and_then(|index| tokens.get(index))
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .and_then(|owner| {
                if tree.get_node_type(owner) == NodeType::Expression {
                    Some(owner)
                } else {
                    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Expression)
                }
            })
            .is_some_and(|owner| {
                let expression_id = LocalNodeId::<Expression>::new(owner);
                matches!(
                    tree.get(expression_id),
                    Expression::If {
                        else_expression: None,
                        ..
                    }
                )
            });

        // normalize eof seams to one trailing newline
        if token_after_index.is_none() || token_after_type == Some(TokenType::End) {
            continue;
        }

        let next_comment = comment_targets
            .iter()
            .find(|(comment_span, _)| comment_span.start >= trivia.span.end)
            .copied();
        let should_skip_semicolon_guard_boundary =
            if let Some((next_comment_span, _)) = next_comment {
                let token_index =
                    tokens.partition_point(|token| token.span.start < next_comment_span.end);
                let next_comment_starts_semicolon_guard_boundary = tokens
                    .iter()
                    .skip(token_index)
                    .find_map(|token| {
                        let token_type = token.token.ty;
                        (!matches!(token_type, TokenType::Whitespace | TokenType::Newline))
                            .then_some(token_type)
                    })
                    .is_some_and(|token_type| {
                        matches!(
                            token_type,
                            TokenType::OpenBracket | TokenType::OpenParenthesis
                        )
                    });

                token_before_is_statement_end
                    && !token_before_is_if_without_else
                    && next_comment_starts_semicolon_guard_boundary
                    && trivia.span.end <= next_comment_span.start
                    && {
                        let token_start_index =
                            tokens.partition_point(|token| token.span.start < trivia.span.end);
                        tokens
                            .iter()
                            .skip(token_start_index)
                            .take_while(|token| token.span.start < next_comment_span.start)
                            .filter(|token| token.span.end > trivia.span.end)
                            .all(|token| token_type_is_whitespace_trivia(token.token.ty))
                    }
            } else {
                false
            };
        if should_skip_semicolon_guard_boundary {
            continue;
        }

        let previous_comment = comment_targets
            .iter()
            .rev()
            .find(|(comment_span, _)| comment_span.end <= trivia.span.start)
            .copied();
        let (target_id, position) =
            blank_trivia_attachment(tree, tokens, trivia, &owner_index, parents);
        let (target_id, position) = if let Some(target_id) = target_id {
            (target_id, position)
        } else {
            let (Some((_, previous_comment_target)), Some((_, next_comment_target))) =
                (previous_comment, next_comment)
            else {
                continue;
            };
            if previous_comment_target != next_comment_target {
                continue;
            }

            (next_comment_target, ast::AnnotationPosition::BlockPrefix)
        };

        if !push_annotation_projection_entry(
            &mut entries,
            &mut by_node_id,
            target_id,
            Annotation::Blank {
                node: trivia.blank,
                position,
            },
            trivia.span,
        ) {
            continue;
        }
    }

    // keep node-local annotation order source-stable
    sort_annotation_projection_by_source(&entries, &mut by_node_id);

    (entries, by_node_id)
}

/// Resolve the default comment trivia rules after specialized seam cases.
fn attach_comment_default(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    trivia: destack_ast::CommentTrivia,
    parents: &NodeParentIndex,
    token_before: Option<usize>,
    token_after: Option<usize>,
    token_before_span: Option<TokenSpan>,
    token_after_span: Option<TokenSpan>,
    has_leading_newline: bool,
    has_trailing_newline: bool,
    comment_is_line: bool,
    enclosing_owner_cache: &mut Option<Option<u32>>,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> (Option<u32>, AnnotationPosition) {
    let is_own_line = has_leading_newline;
    let is_end_of_line = has_trailing_newline || token_after.is_none();
    let following_owner = following_owner
        .or_else(|| {
            token_after.and_then(|token_index| {
                find_owner_at_or_after_token(tree, semantic_tokens, token_index)
            })
        })
        .or_else(|| {
            token_after_span.and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        })
        .map(|target_node| {
            token_after_span
                .map(|token| {
                    promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
                })
                .unwrap_or(target_node)
        })
        .map(|target_node| normalize_trivia_target_owner(tree, target_node));
    let preceding_owner = preceding_owner
        .or_else(|| {
            token_before_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        })
        .map(|target_node| {
            normalize_owner_with_shared_end(
                tree,
                parents,
                target_node,
                token_before_span.map(|token| token.span),
            )
        });

    // ignore directives stay on the following node
    if is_own_line
        && comment_is_line
        && matches!(
            trivia.directive,
            CommentDirective::FormatIgnore
                | CommentDirective::FormatIgnoreFile
                | CommentDirective::FormatIgnoreStart
                | CommentDirective::FormatIgnoreEnd
        )
        && let Some(target_node) = following_owner
    {
        return (Some(target_node), AnnotationPosition::LinePrefix);
    }

    // prettier defaults: own line prefers following, end of line prefers preceding,
    // remaining prefers preceding then following
    if is_own_line {
        if let Some(target_node) = following_owner {
            return (
                Some(target_node),
                if comment_is_line {
                    AnnotationPosition::LinePrefix
                } else {
                    AnnotationPosition::BlockPrefix
                },
            );
        }

        if let Some(target_node) = preceding_owner {
            return (
                Some(target_node),
                if comment_is_line {
                    AnnotationPosition::LinePostfixBoundary
                } else {
                    AnnotationPosition::BlockPostfix
                },
            );
        }
    } else {
        if let Some(target_node) = preceding_owner {
            return (
                Some(target_node),
                if comment_is_line {
                    if is_end_of_line {
                        AnnotationPosition::LinePostfixBoundary
                    } else {
                        AnnotationPosition::LinePostfix
                    }
                } else {
                    AnnotationPosition::BlockPostfix
                },
            );
        }

        if let Some(target_node) = following_owner {
            return (
                Some(target_node),
                if comment_is_line {
                    AnnotationPosition::LinePrefix
                } else {
                    AnnotationPosition::BlockPrefix
                },
            );
        }
    }

    // dangling fallback
    if let Some(enclosing_owner) = comment_enclosing_owner(
        tree,
        token_before_span,
        token_after_span,
        enclosing_owner_cache,
    ) {
        let enclosing_owner = normalize_trivia_target_owner(tree, enclosing_owner);
        return (Some(enclosing_owner), AnnotationPosition::BlockInfix);
    }

    // comment only files can still anchor to one enclosing owner
    if token_before.is_none()
        && token_after.is_none()
        && let Some(target_node) =
            find_smallest_owner_enclosing_range(tree, trivia.span.start, trivia.span.end)
    {
        let target_node = normalize_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    (None, AnnotationPosition::BlockInfix)
}
