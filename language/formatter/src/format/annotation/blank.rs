use destack_ast::{
    AnnotationPosition, Expression, Keyword, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenSpan, TokenType,
};
use destack_source::Span;
use rustc_hash::FxHashMap;

use crate::format::annotation::attachment::{
    FormatterTriviaOwnerIndex, FormatterTriviaSeamIndex, decode_token_index, encode_trivia_seam,
};
use crate::format::annotation::boundary::{
    CommentSeamKeyword, comment_seam_keyword, next_non_trivia_token_index,
    previous_non_newline_token_index, previous_non_trivia_token_index,
};
use crate::format::annotation::ownership::{
    find_owner_at_or_after_token_with_node_type, find_smallest_owner_enclosing_range,
    find_smallest_owner_enclosing_token, lowest_common_owner_ancestor,
    promote_owner_by_shared_start, promote_owner_to_declaration_ancestor,
    promote_owner_to_nearest_statement_boundary, promote_owner_to_node_type_ancestor,
    promote_owner_to_statement_boundary,
};
use crate::format::annotation::semicolon::{token_type_is_comment_trivia, token_type_is_trivia};
use crate::format::annotation::{
    SemicolonGuardCommentSeam, classify_semicolon_guard_comment_seam,
    token_type_is_semicolon_guard_head,
};

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
    token_before_owner: Option<u32>,
    left_owner: Option<u32>,
) -> Option<u32> {
    let owner = if token_before_is_semicolon {
        token_before
            .and_then(|token_index| previous_non_newline_token_index(semantic_tokens, token_index))
            .and_then(|token_index| semantic_tokens.get(token_index))
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .or(left_owner)
    } else {
        token_before_owner.or(left_owner)
    };

    owner.map(|owner_id| promote_owner_to_statement_boundary(tree, parents, owner_id))
}

/// Return whether one seam should be treated as top-level statement spacing noise.
fn seam_is_top_level_statement_spacing_noise(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    left_owner: Option<u32>,
    right_owner: Option<u32>,
    seam_has_comment: bool,
    seam_has_line_comment: bool,
    token_before_is_statement_end: bool,
) -> bool {
    if seam_has_comment || seam_has_line_comment || !token_before_is_statement_end {
        return false;
    }

    let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner) else {
        return false;
    };

    let right_is_top_level =
        promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block).is_none();
    if !right_is_top_level {
        return false;
    }

    let left_type = tree.get_node_type(left_owner);
    let right_type = tree.get_node_type(right_owner);

    matches!(left_type, NodeType::Expression | NodeType::Declaration)
        && matches!(right_type, NodeType::Expression | NodeType::Declaration)
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

pub(crate) fn blank_trivia_attachment(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    trivia: destack_ast::BlankTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    seam_index: &FormatterTriviaSeamIndex,
    parents: &NodeParentIndex,
) -> (Option<u32>, AnnotationPosition) {
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
    let raw_token_before_type = raw_token_before
        .and_then(|index| semantic_tokens.get(index))
        .map(|token| token.token.ty);
    let raw_token_after_type = raw_token_after
        .and_then(|index| semantic_tokens.get(index))
        .map(|token| token.token.ty);
    let token_before = normalized_token_before.or(raw_token_before);
    let token_after = normalized_token_after.or(raw_token_after);
    let (_, _, after_range_comment_start) =
        comment_trivia_summary_in_token_range(semantic_tokens, raw_token_after, token_after);
    let raw_boundary_has_trivia = raw_token_before_type.is_some_and(token_type_is_trivia)
        || raw_token_after_type.is_some_and(token_type_is_trivia);
    let seam_has_comment = seam_index.comment_seams.contains(&seam)
        || (raw_boundary_has_trivia && seam_index.comment_seams.contains(&normalized_seam));
    let seam_has_line_comment = seam_index.line_comment_seams.contains(&seam)
        || (raw_boundary_has_trivia && seam_index.line_comment_seams.contains(&normalized_seam));
    let token_before_span = token_before
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let token_after_span = token_after
        .and_then(|index| semantic_tokens.get(index))
        .copied();

    // seam owners
    let right_owner = token_after
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

    let left_owner = token_before
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

    // file start
    if token_before.is_none() {
        let file_start_has_comment =
            seam_has_comment || raw_token_after_type.is_some_and(token_type_is_comment_trivia);

        if file_start_has_comment && let Some(mut target_node) = right_owner {
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

    // token shape
    let token_before_type = token_before_span.map(|token| token.token.ty);
    let token_after_type = token_after_span.map(|token| token.token.ty);
    let token_after_keyword = comment_seam_keyword(token_keyword_by_span, token_after_span);
    let token_before_keyword = comment_seam_keyword(token_keyword_by_span, token_before_span);

    let token_after_is_else = token_after_keyword == CommentSeamKeyword::Else;
    let token_before_is_else = token_before_keyword == CommentSeamKeyword::Else;
    let token_after_is_at = token_after_type == Some(TokenType::At);
    let token_after_is_semicolon = token_after_type == Some(TokenType::Semicolon);
    let token_after_is_close_brace = token_after_type == Some(TokenType::CloseBrace);
    let token_after_is_open_parenthesis = token_after_type == Some(TokenType::OpenParenthesis);
    let token_after_is_chain_or_index_boundary = matches!(
        token_after_type,
        Some(TokenType::Dot | TokenType::OpenBracket)
    );
    let token_after_comment_next_index = raw_token_after
        .and_then(|token_index| next_non_trivia_token_index(semantic_tokens, token_index));
    let right_owner_after_comment = token_after_comment_next_index
        .and_then(|index| {
            owner_index
                .owner_start_by_token
                .get(index)
                .and_then(|owner| *owner)
        })
        .or_else(|| {
            token_after_comment_next_index.and_then(|index| {
                owner_index
                    .nearest_owner_start_by_token
                    .get(index)
                    .and_then(|owner| *owner)
            })
        });
    let token_after_comment_target_type = token_after_comment_next_index
        .and_then(|token_index| semantic_tokens.get(token_index))
        .map(|token| token.token.ty);
    let token_after_comment_continues_chain_or_index = matches!(
        token_after_type,
        Some(TokenType::Dot | TokenType::OpenBracket)
    ) || matches!(
        (token_after_type, token_after_comment_target_type),
        (Some(comment_type), Some(TokenType::Dot | TokenType::OpenBracket))
            if token_type_is_comment_trivia(comment_type)
    );
    let token_before_is_semicolon = token_before_type == Some(TokenType::Semicolon);
    let token_before_is_open_parenthesis = token_before_type == Some(TokenType::OpenParenthesis);
    let token_before_is_comma = token_before_type == Some(TokenType::Comma);
    let token_before_is_colon = token_before_type == Some(TokenType::Colon);
    let token_before_is_statement_end = matches!(
        token_before_type,
        Some(TokenType::Semicolon | TokenType::CloseBrace | TokenType::CloseParenthesis)
    );

    let token_before_owner =
        token_before_span.and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let left_owner_before_semicolon = owner_before_semicolon(
        tree,
        parents,
        semantic_tokens,
        token_before,
        token_before_is_semicolon,
        token_before_owner,
        left_owner,
    );
    let semicolon_guard_seam = classify_semicolon_guard_comment_seam(
        semantic_tokens,
        token_before_type,
        token_after_type,
        token_after,
    );
    let semicolon_guard_starter_before_comment = matches!(
        semicolon_guard_seam,
        SemicolonGuardCommentSeam::BeforeComment { .. }
    );
    let semicolon_after_comment_target_type = match semicolon_guard_seam {
        SemicolonGuardCommentSeam::AfterComment { target_type } => Some(target_type),
        _ => None,
    };
    let semicolon_guard_starter_after_comment =
        semicolon_after_comment_target_type.is_some_and(token_type_is_semicolon_guard_head);
    let is_if_semicolon_guard_array_separator_seam =
        owner_has_if_statement_ancestor(tree, parents, left_owner_before_semicolon)
            && ((token_after_is_semicolon
                && semicolon_after_comment_target_type == Some(TokenType::OpenBracket))
                || (token_before_is_semicolon && token_after_type == Some(TokenType::OpenBracket)));

    // owner topology
    let right_owner_is_argument =
        right_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Argument);
    let shared_expression_owner = left_owner
        .zip(right_owner)
        .and_then(|(left_owner, right_owner)| {
            lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
        })
        .and_then(|owner| (tree.get_node_type(owner) == NodeType::Expression).then_some(owner));
    let seam_has_shared_expression_owner = shared_expression_owner.is_some();
    let left_match_case_owner = left_owner.and_then(|owner| {
        promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::MatchCase)
    });

    // comment seam shape
    let blank_before_first_comment = seam_index
        .first_comment_start_by_seam
        .get(&seam)
        .copied()
        .or_else(|| {
            seam_index
                .first_comment_start_by_seam
                .get(&normalized_seam)
                .copied()
        })
        .is_some_and(|start| trivia.span.end <= start);
    let blank_before_first_comment_in_after_range =
        after_range_comment_start.is_some_and(|start| trivia.span.end <= start);

    // statement-end chain seams: keep pre-comment blank lines on the left statement boundary
    if (blank_before_first_comment || blank_before_first_comment_in_after_range)
        && token_before_is_statement_end
        && token_after_comment_continues_chain_or_index
        && matches!(semicolon_guard_seam, SemicolonGuardCommentSeam::None)
        && let Some(target_node) = token_before_owner
            .or(left_owner)
            .or(left_owner_before_semicolon)
    {
        return block_postfix_attachment(tree, target_node);
    }

    // switch label seams: keep pre-comment blank before `case` labels
    if token_before_is_colon
        && seam_has_comment
        && blank_before_first_comment
        && let Some(target_node) = left_match_case_owner
    {
        return block_postfix_attachment(tree, target_node);
    }

    // else seams: keep pre-comment blank, drop post-comment blank
    if seam_has_comment && token_after_is_else {
        if blank_before_first_comment && let Some(target_node) = shared_expression_owner {
            return block_postfix_attachment(tree, target_node);
        }

        if blank_before_first_comment && let Some(mut target_node) = right_owner {
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

    // if semicolon-guard array seams preserve one pre-comment blank line before the guard
    if seam_has_comment
        && blank_before_first_comment
        && is_if_semicolon_guard_array_separator_seam
        && let Some(target_node) = right_owner.or(left_owner_before_semicolon).or(left_owner)
    {
        return block_prefix_attachment(tree, target_node);
    }

    // semicolon guard seams: preserve one pre-comment blank before semicolon-after guards
    if seam_has_comment
        && blank_before_first_comment
        && semicolon_guard_starter_after_comment
        && let Some(target_node) = right_owner.or(left_owner_before_semicolon).or(left_owner)
    {
        return block_prefix_attachment(tree, target_node);
    }

    // semicolon guard seams: other blank trivia does not carry independent structure
    if seam_has_comment
        && (semicolon_guard_starter_before_comment || semicolon_guard_starter_after_comment)
    {
        return blank_infix_attachment();
    }

    // statement-end seams: pre-comment blank lines stay on the left statement boundary
    if seam_has_line_comment && blank_before_first_comment && token_before_is_statement_end {
        if token_after_comment_continues_chain_or_index
            && let Some(target_node) = token_before_owner
                .or(left_owner_before_semicolon)
                .or(left_owner)
        {
            return block_postfix_attachment(tree, target_node);
        }

        if let Some(target_node) = right_owner.or(right_owner_after_comment) {
            let target_node =
                promote_owner_to_nearest_statement_boundary(tree, parents, target_node);
            return block_prefix_attachment(tree, target_node);
        }

        if let Some(target_node) = left_owner_before_semicolon
            .or(token_before_owner)
            .or(left_owner)
        {
            return block_postfix_attachment(tree, target_node);
        }
    }

    // statement-end seams: pre-comment blank lines stay on the left statement boundary
    if seam_has_comment
        && !seam_has_line_comment
        && blank_before_first_comment
        && token_before_is_statement_end
        && let Some(target_node) = left_owner_before_semicolon
            .or(token_before_owner)
            .or(left_owner)
    {
        return block_postfix_attachment(tree, target_node);
    }

    // chain and index seams with comments: preserve around-comment blank ownership
    if seam_has_comment && token_after_is_chain_or_index_boundary {
        if blank_before_first_comment && let Some(target_node) = left_owner {
            return block_postfix_attachment(tree, target_node);
        }

        if let Some(token) = token_after_span
            && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token.span)
        {
            return block_prefix_attachment(tree, target_node);
        }
    }

    // semicolon-own-line comment seams: pre-comment blank is formatting noise
    if seam_has_comment
        && blank_before_first_comment
        && token_before_is_semicolon
        && token_after_is_semicolon
    {
        return blank_infix_attachment();
    }

    // comma seams: pre-comment blank belongs to the right owner
    if seam_has_comment
        && blank_before_first_comment
        && token_before_is_comma
        && let Some(target_node) = right_owner
    {
        return block_prefix_attachment(tree, target_node);
    }

    // semicolon and block terminator seams do not carry independent blank trivia
    if token_after_is_semicolon || token_after_is_close_brace {
        return blank_infix_attachment();
    }

    // else-to-body seams do not carry independent blank trivia
    if token_before_is_else {
        return blank_infix_attachment();
    }

    // decorator seams: keep blanks with the decorated declaration or member
    if token_after_is_at {
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
            && let Some(left_declaration) = left_owner
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            && let Some(target_declaration) =
                promote_owner_to_declaration_ancestor(tree, parents, target_node)
            && left_declaration == target_declaration
        {
            return block_prefix_attachment(tree, target_node);
        }

        if token_before_is_semicolon
            && let Some(target_node) = left_owner
            && tree.get_node_type(target_node) == NodeType::Member
        {
            return block_postfix_attachment(tree, target_node);
        }

        if let Some(token) = token_after_span
            && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token.span)
            && tree.get_node_type(target_node) == NodeType::Member
        {
            return block_prefix_attachment(tree, target_node);
        }

        if let Some(target_node) = right_owner
            && tree.get_node_type(target_node) == NodeType::Member
        {
            return block_prefix_attachment(tree, target_node);
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
                right_owner
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
                    .or(right_owner)
            });
        if let Some(target_node) = declaration_target {
            return block_prefix_attachment(tree, target_node);
        }
    }

    // top-level statement seams are already controlled by statement-list formatting
    if seam_is_top_level_statement_spacing_noise(
        tree,
        parents,
        left_owner,
        right_owner,
        seam_has_comment,
        seam_has_line_comment,
        token_before_is_statement_end,
    ) {
        return blank_infix_attachment();
    }

    // assignment seams before parenthesized rhs values keep blank on the rhs owner
    if token_before_type == Some(TokenType::Assign)
        && token_after_type == Some(TokenType::OpenParenthesis)
        && let Some(mut target_node) = right_owner
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

    // argument list delimiter seams do not carry independent blank trivia
    if right_owner_is_argument && token_after_is_open_parenthesis {
        return blank_infix_attachment();
    }

    if right_owner_is_argument && token_before_is_open_parenthesis {
        return blank_infix_attachment();
    }

    // chain and index seams without comments stay on the left segment
    if token_after_is_chain_or_index_boundary
        && seam_has_shared_expression_owner
        && !token_before_is_comma
        && let Some(target_node) = left_owner
    {
        return block_postfix_attachment(tree, target_node);
    }

    // default: attach to right owner when available
    if let Some(target_node) = right_owner {
        let target_node = token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        return block_prefix_attachment(tree, target_node);
    }

    // fallback: attach to left owner
    if let Some(target_node) = left_owner {
        return block_postfix_attachment(tree, target_node);
    }

    // comment-only files still need one structural owner for blank trivia
    if let Some(target_node) =
        find_smallest_owner_enclosing_range(tree, trivia.span.start, trivia.span.end)
    {
        return block_prefix_attachment(tree, target_node);
    }

    blank_infix_attachment()
}
