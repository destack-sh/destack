use ast::{
    AnnotationPosition, Expression, Keyword, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
    TokenSpan, TokenType,
};
use destack_ast as ast;
use destack_source::Span;
use rustc_hash::FxHashMap;

use crate::format::comments::attachment::{
    FormatterTriviaOwnerIndex, FormatterTriviaSeamIndex, decode_token_index, encode_trivia_seam,
};
use crate::format::comments::boundary::{CommentSeamKeyword, comment_seam_keyword};
use crate::format::comments::ownership::{
    find_next_declaration_owner_from_token, find_next_member_owner_from_token,
    find_smallest_owner_enclosing_range, find_smallest_owner_enclosing_token,
    lowest_common_owner_ancestor, normalize_formatter_trivia_target_owner,
    promote_owner_by_shared_start, promote_owner_to_declaration_ancestor,
    promote_owner_to_node_type_ancestor,
};

pub(crate) fn blank_trivia_attachment(
    tree: &NodeTree,
    semantic_tokens: &[TokenSpan],
    token_keyword_by_span: &FxHashMap<Span, Option<Keyword>>,
    trivia: destack_ast::BlankTrivia,
    owner_index: &FormatterTriviaOwnerIndex,
    seam_index: &FormatterTriviaSeamIndex,
    parents: &NodeParentIndex,
) -> (Option<u32>, AnnotationPosition) {
    let token_before = decode_token_index(trivia.boundary.token_before);
    let token_after = decode_token_index(trivia.boundary.token_after);
    let token_before_span = token_before
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let token_after_span = token_after
        .and_then(|index| semantic_tokens.get(index))
        .copied();
    let seam = encode_trivia_seam(trivia.boundary.token_before, trivia.boundary.token_after);
    let seam_has_comment = seam_index.comment_seams.contains(&seam);
    let seam_has_line_comment = seam_index.line_comment_seams.contains(&seam);

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

    // blank trivia at file start should not produce leading empty lines
    if token_before.is_none() {
        if seam_has_comment && let Some(mut target_node) = right_owner {
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

            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        return (None, AnnotationPosition::BlockInfix);
    }

    let token_after_is_at = token_after_span.is_some_and(|token| token.token.ty == TokenType::At);
    let token_after_is_semicolon =
        token_after_span.is_some_and(|token| token.token.ty == TokenType::Semicolon);
    let token_after_is_close_brace =
        token_after_span.is_some_and(|token| token.token.ty == TokenType::CloseBrace);
    let token_after_is_open_parenthesis =
        token_after_span.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis);
    let token_after_is_chain_or_index_boundary = token_after_span
        .is_some_and(|token| matches!(token.token.ty, TokenType::Dot | TokenType::OpenBracket));
    let token_after_is_else =
        comment_seam_keyword(token_keyword_by_span, token_after_span) == CommentSeamKeyword::Else;
    let token_before_is_else =
        comment_seam_keyword(token_keyword_by_span, token_before_span) == CommentSeamKeyword::Else;
    let token_before_is_open_parenthesis =
        token_before_span.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis);
    let token_before_is_comma =
        token_before_span.is_some_and(|token| token.token.ty == TokenType::Comma);
    let token_before_is_statement_end = token_before_span.is_some_and(|token| {
        matches!(
            token.token.ty,
            TokenType::Semicolon | TokenType::CloseBrace | TokenType::CloseParenthesis
        )
    });
    let token_after_starts_statement = token_after_span.is_some_and(|token| {
        matches!(
            token.token.ty,
            TokenType::Identifier | TokenType::At | TokenType::OpenParenthesis
        )
    });
    let right_owner_is_argument =
        right_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Argument);
    let shared_expression_owner = left_owner
        .zip(right_owner)
        .and_then(|(left_owner, right_owner)| {
            lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
        })
        .and_then(|owner| (tree.get_node_type(owner) == NodeType::Expression).then_some(owner));
    let seam_has_shared_expression_owner = shared_expression_owner.is_some();
    let blank_before_first_comment = seam_index
        .first_comment_start_by_seam
        .get(&seam)
        .copied()
        .is_some_and(|start| trivia.span.end <= start);

    // blank seams around comments before `else` are asymmetric:
    // keep true pre-comment blank lines, drop post-comment blanks
    if seam_has_comment && token_after_is_else {
        if blank_before_first_comment && let Some(target_node) = shared_expression_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPostfix);
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

            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        return (None, AnnotationPosition::BlockInfix);
    }

    // blank seams around chain-boundary comments should preserve pre and post comment spacing
    if seam_has_comment && token_after_is_chain_or_index_boundary {
        if blank_before_first_comment && let Some(target_node) = left_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPostfix);
        }

        if let Some(token) = token_after_span
            && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token.span)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }
    }

    // blank seams before the first comment in one seam stay on the left owner
    if seam_has_comment
        && blank_before_first_comment
        && let Some(target_node) = left_owner
        && tree.get_node_type(target_node) != NodeType::Block
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // blank seams before semicolons or closing braces are formatting noise
    if token_after_is_semicolon || token_after_is_close_brace {
        return (None, AnnotationPosition::BlockInfix);
    }

    // blank seams between `else` and branch bodies are formatting noise
    if token_before_is_else {
        return (None, AnnotationPosition::BlockInfix);
    }

    // blanks before decorators should stay before the decorated declaration
    if token_after_is_at {
        if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
            && let Some(token_after_index) = token_after
            && let Some(target_node) =
                find_next_member_owner_from_token(tree, owner_index, token_after_index)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
            && let Some(target_node) = left_owner
            && tree.get_node_type(target_node) == NodeType::Member
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPostfix);
        }

        if let Some(token) = token_after_span
            && let Some(target_node) = find_smallest_owner_enclosing_token(tree, token.span)
            && tree.get_node_type(target_node) == NodeType::Member
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        if let Some(target_node) = right_owner
            && tree.get_node_type(target_node) == NodeType::Member
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }

        let declaration_target = token_after
            .and_then(|token_after_index| {
                find_next_declaration_owner_from_token(tree, owner_index, token_after_index)
            })
            .or_else(|| {
                right_owner
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
                    .or(right_owner)
            });
        if let Some(target_node) = declaration_target {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return (Some(target_node), AnnotationPosition::BlockPrefix);
        }
    }

    // top-level expression seams already carry spacing in statement-list formatting
    if token_before_is_statement_end
        && token_after_starts_statement
        && !seam_has_line_comment
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Expression
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_none()
    {
        return (None, AnnotationPosition::BlockInfix);
    }

    // top-level expression-to-declaration seams already carry spacing in statement-list formatting
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
        && !seam_has_line_comment
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Expression
        && tree.get_node_type(right_owner) == NodeType::Declaration
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_none()
    {
        return (None, AnnotationPosition::BlockInfix);
    }

    // top-level declaration-to-expression seams after close braces don't need blank trivia
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::CloseBrace)
        && !seam_has_line_comment
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Declaration
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_none()
    {
        return (None, AnnotationPosition::BlockInfix);
    }

    // block-local seams before await expressions don't need extra blank trivia
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
        && !seam_has_line_comment
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(left_owner) == NodeType::Expression
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_some()
    {
        let right_expression = LocalNodeId::<Expression>::new(right_owner);
        if matches!(tree.get(right_expression), Expression::Await { .. }) {
            return (None, AnnotationPosition::BlockInfix);
        }
    }

    // semicolon seams before block-local member-path expressions don't carry independent blank trivia
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::Semicolon)
        && !seam_has_line_comment
        && let Some(right_owner) = right_owner
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_some()
    {
        let right_expression = LocalNodeId::<Expression>::new(right_owner);
        if let Expression::Path { path, .. } = tree.get(right_expression)
            && path.segments.len() > 1
        {
            return (None, AnnotationPosition::BlockInfix);
        }
    }

    // top-level seams after declaration close braces and before plain expressions
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::CloseBrace)
        && !seam_has_line_comment
        && let (Some(left_owner), Some(right_owner)) = (left_owner, right_owner)
        && tree.get_node_type(right_owner) == NodeType::Expression
        && promote_owner_to_node_type_ancestor(tree, parents, right_owner, NodeType::Block)
            .is_none()
        && promote_owner_to_declaration_ancestor(tree, parents, left_owner).is_some()
    {
        let right_expression = LocalNodeId::<Expression>::new(right_owner);
        if !matches!(tree.get(right_expression), Expression::Export { .. }) {
            return (None, AnnotationPosition::BlockInfix);
        }
    }

    // assignment seams should keep blank separators with the rhs value owner
    if token_before_span.is_some_and(|token| token.token.ty == TokenType::Assign)
        && token_after_span.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis)
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

        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    // blank seams between callees and argument lists are formatting noise
    if right_owner_is_argument && token_after_is_open_parenthesis {
        return (None, AnnotationPosition::BlockInfix);
    }

    // blank seams right after `(` before first arguments are formatting noise
    if right_owner_is_argument && token_before_is_open_parenthesis {
        return (None, AnnotationPosition::BlockInfix);
    }

    // blank seams before chain and index operators stay with the left segment
    if token_after_is_chain_or_index_boundary
        && seam_has_shared_expression_owner
        && !token_before_is_comma
        && let Some(target_node) = left_owner
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    if let Some(target_node) = right_owner {
        let target_node = token_after_span
            .map(|token| {
                promote_owner_by_shared_start(tree, parents, target_node, token.span.start)
            })
            .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    if let Some(target_node) = left_owner {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPostfix);
    }

    // comment-only files still need one structural owner for blank trivia
    if let Some(target_node) =
        find_smallest_owner_enclosing_range(tree, trivia.span.start, trivia.span.end)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return (Some(target_node), AnnotationPosition::BlockPrefix);
    }

    (None, AnnotationPosition::BlockInfix)
}
