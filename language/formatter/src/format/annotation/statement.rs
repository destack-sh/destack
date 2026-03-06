use destack_ast::{
    AnnotationPosition, Block, BlockFormat, Expression, LocalNodeId, MatchCase, MatchSelector,
    NodeParentIndex, NodeTree, NodeType, TokenSpan, TokenType, WhileKind,
};

use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner,
};
use super::declaration::try_attach_comment_declaration_return_type_seam;
use super::facts::{next_non_trivia_token_index, previous_non_trivia_token_index};
use super::ownership::{
    block_leading_comment_target, find_smallest_owner_enclosing_token,
    lowest_common_owner_ancestor, normalize_formatter_trivia_target_owner,
    promote_owner_to_node_type_ancestor,
};
use super::semicolon::{
    attach_semicolon_guard_own_line_comment, try_attach_comment_before_empty_statement_semicolon,
};

/// Normalize one owner to its parameter or argument container owner.
fn normalize_parameter_or_argument_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> u32 {
    promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Parameter)
        .or_else(|| {
            promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Argument)
        })
        .or_else(|| dynamic_argument_owner_for_call_like_ancestor(tree, parents, owner_id))
        .unwrap_or(owner_id)
}

/// Normalize one owner to its argument container owner.
fn normalize_argument_owner(tree: &NodeTree, parents: &NodeParentIndex, owner_id: u32) -> u32 {
    promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Argument)
        .or_else(|| dynamic_argument_owner_for_call_like_ancestor(tree, parents, owner_id))
        .unwrap_or(owner_id)
}

/// Return one last dynamic argument owner for one call-like expression owner.
fn last_dynamic_argument_owner_for_call_like(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    match tree.get(expression_id) {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments.last().map(|argument_id| argument_id.id),
        Expression::Import { arguments, .. } => arguments.as_ref().and_then(|dynamic_arguments| {
            dynamic_arguments.last().map(|argument_id| argument_id.id)
        }),
        _ => None,
    }
}

/// Resolve one last argument owner from call-like expression ancestry.
fn dynamic_argument_owner_for_call_like_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_owner = Some(owner_id);
    while let Some(current_id) = current_owner {
        if tree.get_node_type(current_id) == NodeType::Expression
            && let Some(argument_owner) =
                last_dynamic_argument_owner_for_call_like(tree, current_id)
        {
            return Some(argument_owner);
        }

        current_owner = parents.get_by_id(current_id);
    }

    None
}

/// Return whether one argument owner belongs to one call-like expression.
fn owner_is_call_like_argument(tree: &NodeTree, parents: &NodeParentIndex, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Argument {
        return false;
    }

    let Some(parent_id) = parents.get_by_id(owner_id) else {
        return false;
    };
    if tree.get_node_type(parent_id) != NodeType::Expression {
        return false;
    }

    matches!(
        tree.get(LocalNodeId::<Expression>::new(parent_id)),
        Expression::Call { .. } | Expression::New { .. } | Expression::Import { .. }
    )
}

/// Resolve one separator seam owner from preceding owners and token boundaries.
fn separator_preceding_owner(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
) -> Option<u32> {
    if let Some(preceding_owner) = preceding_owner {
        return Some(preceding_owner);
    }

    if seam.token_before_is(TokenType::Comma)
        && let Some(comma_token_index) = context.token_before
        && let Some(previous_token_index) =
            previous_non_trivia_token_index(context.semantic_tokens, comma_token_index)
        && let Some(previous_token) = context.semantic_tokens.get(previous_token_index)
        && let Some(owner_id) = find_smallest_owner_enclosing_token(tree, previous_token.span)
    {
        return Some(owner_id);
    }

    context
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
}

/// Return whether one owner token sits inside one decorator expression ancestry.
fn seam_follows_decorator_head(context: &CommentSeamContext<'_>) -> bool {
    let Some(mut token_index) = context.token_before else {
        return false;
    };

    loop {
        let token_type = context.semantic_tokens[token_index].token.ty;
        if token_type == TokenType::At {
            return true;
        }

        if token_type == TokenType::Newline || token_index == 0 {
            return false;
        }

        token_index -= 1;
    }
}

/// Resolve one else-body owner for comments between `else` and its body.
fn else_body_comment_target_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    following_owner: Option<u32>,
) -> Option<u32> {
    let target_owner = following_owner?;
    let target_owner = if tree.get_node_type(target_owner) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(target_owner);
        if let Expression::Block(block_id) = tree.get(expression_id) {
            block_id.id
        } else {
            target_owner
        }
    } else {
        target_owner
    };
    let target_owner =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Block)
            .unwrap_or(target_owner);

    Some(target_owner)
}

/// Attach one separator comment before `)` to its parameter or argument owner.
fn seam_has_separator_before_close_parenthesis(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    if seam.token_after_is(TokenType::CloseParenthesis) {
        return true;
    }

    seam.token_after_is(TokenType::Comma)
        && context
            .token_after
            .and_then(|token_index| {
                next_non_trivia_token_index(context.semantic_tokens, token_index)
            })
            .and_then(|token_index| context.semantic_tokens.get(token_index))
            .is_some_and(|token| token.token.ty == TokenType::CloseParenthesis)
}

/// Return whether one seam comment can bind to one separator before `)`.
fn seam_supports_separator_comment(seam: &CommentSeamData) -> bool {
    seam.comment_is_line
        || (seam.comment_is_star && seam.has_leading_newline && seam.has_trailing_newline)
}

/// Attach one separator comment before `)` to its parameter or argument owner.
fn try_attach_parameter_or_argument_separator_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    target_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam_has_separator_before_close_parenthesis(context, seam) {
        return None;
    }

    // separator seams include line comments and own-line block comments
    if !seam_supports_separator_comment(seam) {
        return None;
    }

    let target_owner = target_owner?;
    let target_owner = normalize_parameter_or_argument_owner(tree, parents, target_owner);
    let target_owner_type = tree.get_node_type(target_owner);
    if target_owner_type != NodeType::Parameter && target_owner_type != NodeType::Argument {
        return None;
    }

    // argument separator seams only apply to call/new argument lists
    if target_owner_type == NodeType::Argument
        && !owner_is_call_like_argument(tree, parents, target_owner)
    {
        return None;
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve one own line case/default prefix comment target.
fn case_or_default_prefix_target(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if let Some(following_owner) = following_owner
        && tree.get_node_type(following_owner) == NodeType::MatchCase
    {
        return Some((Some(following_owner), AnnotationPosition::BlockPrefix));
    }

    let target_owner =
        comment_enclosing_owner(context, enclosing_owner_cache).or(following_owner)?;

    if tree.get_node_type(target_owner) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(target_owner);
        if let Expression::Match { cases, .. } = tree.get(expression_id)
            && let Some(first_case) = cases.first().copied()
        {
            return Some((Some(first_case.id), AnnotationPosition::BlockPrefix));
        }
    }

    if tree.get_node_type(target_owner) == NodeType::Block {
        let block_id = LocalNodeId::<Block>::new(target_owner);
        let (target_owner, position) = block_leading_comment_target(tree, block_id);
        return Some((Some(target_owner), position));
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Return the match-case ancestor owner for one optional owner id.
fn owner_match_case_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner: Option<u32>,
) -> Option<u32> {
    owner.and_then(|owner_id| {
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::MatchCase)
    })
}

/// Resolve one switch-label seam owner from neighboring owners.
fn switch_label_comment_match_case_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<u32> {
    owner_match_case_ancestor(tree, parents, preceding_owner)
        .or_else(|| owner_match_case_ancestor(tree, parents, following_owner))
        .or_else(|| {
            context
                .token_before_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
                .and_then(|owner| {
                    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::MatchCase)
                })
        })
        .or_else(|| {
            context
                .token_after_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
                .and_then(|owner| {
                    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::MatchCase)
                })
        })
}

/// Return the explicit block consequent owner for one match case.
fn explicit_match_case_consequent_block_owner(
    tree: &NodeTree,
    match_case_owner: u32,
) -> Option<LocalNodeId<Block>> {
    if tree.get_node_type(match_case_owner) != NodeType::MatchCase {
        return None;
    }

    let match_case_id = LocalNodeId::<MatchCase>::new(match_case_owner);
    match tree.get(match_case_id) {
        MatchCase::Expression { body, .. } => {
            let Expression::Block(block_id) = tree.get(*body) else {
                return None;
            };
            let block = tree.get(*block_id);
            if block.format != BlockFormat::Implicit {
                return Some(*block_id);
            }
        }
        MatchCase::Block { body, .. } => {
            let block = tree.get(*body);
            if block.format != BlockFormat::Implicit {
                return Some(*body);
            }
        }
    }

    None
}

/// Return one switch-label explicit block leading attachment for comments before `{`.
fn switch_label_explicit_block_attachment(
    tree: &NodeTree,
    match_case_owner: u32,
    seam: &CommentSeamData,
) -> Option<CommentAttachment> {
    if !seam.token_after_is(TokenType::OpenBrace) {
        return None;
    }

    let explicit_block_owner = explicit_match_case_consequent_block_owner(tree, match_case_owner)?;
    let (target_owner, position) = block_leading_comment_target(tree, explicit_block_owner);
    Some((Some(target_owner), position))
}

/// Return whether one match-case owner is the `default` selector case.
fn match_case_owner_is_default(tree: &NodeTree, match_case_owner: u32) -> bool {
    if tree.get_node_type(match_case_owner) != NodeType::MatchCase {
        return false;
    }

    let match_case_id = LocalNodeId::<MatchCase>::new(match_case_owner);
    let selector = match tree.get(match_case_id) {
        MatchCase::Expression { selector, .. } | MatchCase::Block { selector, .. } => selector,
    };
    matches!(selector, MatchSelector::Default)
}

/// Promote one owner to the nearest control-head expression ancestor.
fn promote_owner_to_control_head_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<LocalNodeId<Expression>> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            match tree.get(expression_id) {
                Expression::If { .. }
                | Expression::ForEach { .. }
                | Expression::For { .. }
                | Expression::Loop { .. } => return Some(expression_id),
                Expression::While { kind, .. } if *kind != WhileKind::DoWhile => {
                    return Some(expression_id);
                }
                _ => {}
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Resolve one comment after a control-head `)` target.
fn control_head_comment_attachment_for_expression(
    tree: &NodeTree,
    control_expression: LocalNodeId<Expression>,
) -> Option<CommentAttachment> {
    match tree.get(control_expression) {
        Expression::If {
            then_expression, ..
        } => {
            if matches!(tree.get(*then_expression), Expression::Block(_)) {
                let Expression::Block(block_id) = tree.get(*then_expression) else {
                    unreachable!();
                };
                let (target_owner, position) = block_leading_comment_target(tree, *block_id);
                return Some((Some(target_owner), position));
            }

            Some((Some(then_expression.id), AnnotationPosition::BlockPrefix))
        }
        Expression::While { body, .. }
        | Expression::ForEach { body, .. }
        | Expression::For { body, .. }
        | Expression::Loop { body } => {
            let (target_owner, position) = block_leading_comment_target(tree, *body);
            Some((Some(target_owner), position))
        }
        _ => None,
    }
}

/// Resolve one comment after a control-head token span.
fn control_head_comment_target_from_token(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    token_before_span: Option<TokenSpan>,
) -> Option<CommentAttachment> {
    let token_before_span = token_before_span?;
    let owner_id = find_smallest_owner_enclosing_token(tree, token_before_span.span)?;
    let control_expression =
        promote_owner_to_control_head_expression_ancestor(tree, parents, owner_id)?;

    control_head_comment_attachment_for_expression(tree, control_expression)
}

/// Resolve one comment after a control-head `)` target.
fn control_head_comment_target(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    let shared_owner =
        preceding_owner
            .zip(following_owner)
            .and_then(|(preceding_owner, following_owner)| {
                lowest_common_owner_ancestor(tree, parents, preceding_owner, following_owner)
            });

    let control_expression = [shared_owner, preceding_owner]
        .into_iter()
        .flatten()
        .find_map(|owner_id| {
            promote_owner_to_control_head_expression_ancestor(tree, parents, owner_id)
        })?;

    control_head_comment_attachment_for_expression(tree, control_expression)
}

/// Resolve own-line comments between decorators and decorated items.
fn try_attach_decorator_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    // own-line comments that start on a new line after the preceding token are decorator seams
    let comment_starts_after_token_before_newline =
        context.token_before_span.is_some_and(|token| {
            context
                .file
                .is_same_line(token.span.end.saturating_sub(1), context.trivia.span.start)
        });

    if !seam.comment_is_line || comment_starts_after_token_before_newline {
        return None;
    }

    if !seam_follows_decorator_head(context) {
        return None;
    }

    // decorated items can be parameter, member, property, or declaration owners
    let target_owner = following_owner
        .or_else(|| {
            context
                .token_after_span
                .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        })
        .and_then(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Parameter)
                .or_else(|| {
                    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
                })
                .or_else(|| {
                    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Property)
                })
                .or_else(|| {
                    promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Declaration)
                })
        })?;

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Resolve own-line separator comments before `)`.
fn try_attach_separator_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline {
        return None;
    }

    let separator_owner = separator_preceding_owner(tree, context, seam, preceding_owner);
    try_attach_parameter_or_argument_separator_comment(
        tree,
        parents,
        context,
        seam,
        separator_owner,
    )
}

/// Resolve comments before empty-statement body semicolons.
fn try_attach_empty_statement_semicolon_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.token_after_is(TokenType::Semicolon) {
        return None;
    }

    try_attach_comment_before_empty_statement_semicolon(
        tree,
        parents,
        context,
        seam,
        preceding_owner,
        following_owner,
    )
}

/// Resolve own-line comments between `else` and its body.
fn try_attach_else_body_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline || !seam.token_before_is_keyword(CommentSeamKeyword::Else) {
        return None;
    }

    let target_owner = else_body_comment_target_owner(tree, parents, following_owner)?;
    let position = if tree.get_node_type(target_owner) == NodeType::Block {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };

    Some((Some(target_owner), position))
}

/// Resolve own-line comments after control heads.
fn try_attach_control_head_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline {
        return None;
    }

    if !seam.token_before_is(TokenType::CloseParenthesis) || seam.token_after_is_case_or_default() {
        return None;
    }

    let supports_control_head_seam = seam.comment_is_line
        || seam.token_after_is(TokenType::Semicolon)
        || !seam.token_after_is(TokenType::OpenBrace);
    if !supports_control_head_seam {
        return None;
    }

    control_head_comment_target_from_token(tree, parents, context.token_before_span)
        .or_else(|| control_head_comment_target(tree, parents, preceding_owner, following_owner))
}

/// Resolve own-line comments after switch labels before case bodies.
fn try_attach_switch_label_body_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline
        || !seam.token_before_is(TokenType::Colon)
        || seam.token_after_is_case_or_default()
    {
        return None;
    }

    let match_case_owner = switch_label_comment_match_case_owner(
        tree,
        parents,
        context,
        preceding_owner,
        following_owner,
    )?;

    if let Some(attachment) = switch_label_explicit_block_attachment(tree, match_case_owner, seam) {
        return Some(attachment);
    }

    let following_match_case_owner = owner_match_case_ancestor(tree, parents, following_owner);
    if following_match_case_owner == Some(match_case_owner)
        && let Some(target_owner) = following_owner
    {
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        let position = if seam.comment_is_line {
            AnnotationPosition::LinePrefix
        } else {
            AnnotationPosition::BlockPrefix
        };
        return Some((Some(target_owner), position));
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, match_case_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve own-line comments between switch labels.
fn try_attach_switch_label_next_case_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline
        || !seam.token_before_is(TokenType::Colon)
        || !seam.token_after_is_case_or_default()
    {
        return None;
    }

    let target_owner = context
        .token_after_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
        .and_then(|owner_id| {
            promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::MatchCase)
                .or(Some(owner_id))
        })
        .map(|owner_id| normalize_formatter_trivia_target_owner(tree, owner_id))?;

    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Resolve own-line comments before switch `case` and `default` labels.
fn try_attach_switch_case_prefix_own_line_comment(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline || !seam.token_after_is_case_or_default() {
        return None;
    }

    case_or_default_prefix_target(tree, context, enclosing_owner_cache, following_owner)
}

/// Resolve inline line comments between `else` and its body.
fn try_attach_else_body_inline_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline
        || !seam.comment_is_line
        || !seam.token_before_is_keyword(CommentSeamKeyword::Else)
    {
        return None;
    }

    let target_owner = else_body_comment_target_owner(tree, parents, following_owner)?;
    let position = if tree.get_node_type(target_owner) == NodeType::Block {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };
    Some((Some(target_owner), position))
}

/// Resolve inline star comments between `else` and its body.
fn try_attach_else_body_inline_star_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline
        || !seam.comment_is_star
        || !seam.token_before_is_keyword(CommentSeamKeyword::Else)
    {
        return None;
    }

    let token_after_is_comment = matches!(
        seam.token_after_type,
        Some(
            TokenType::LineComment
                | TokenType::BlockComment
                | TokenType::DocLineComment
                | TokenType::DocBlockComment
        )
    );
    if token_after_is_comment {
        return None;
    }

    let target_owner = else_body_comment_target_owner(tree, parents, following_owner)?;
    if tree.get_node_type(target_owner) == NodeType::Block {
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Resolve same-line comments after control heads.
fn try_attach_control_head_inline_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline {
        return None;
    }

    if !seam.token_before_is(TokenType::CloseParenthesis) || seam.token_after_is_case_or_default() {
        return None;
    }

    let supports_control_head_seam = seam.comment_is_line
        || seam.token_after_is(TokenType::Semicolon)
        || !seam.token_after_is(TokenType::OpenBrace);
    if !supports_control_head_seam {
        return None;
    }

    let attachment =
        control_head_comment_target_from_token(tree, parents, context.token_before_span).or_else(
            || control_head_comment_target(tree, parents, preceding_owner, following_owner),
        )?;

    // inline star comments before non-block control bodies become line prefixes
    if seam.comment_is_star
        && !seam.has_trailing_newline
        && !seam.token_after_is(TokenType::Semicolon)
        && let Some(target_owner) = attachment.0
        && tree.get_node_type(target_owner) != NodeType::Block
    {
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    Some(attachment)
}

/// Resolve trailing line comments after `switch (...) {`.
fn try_attach_switch_header_trailing_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline
        || !seam.has_trailing_newline
        || !seam.comment_is_line
        || !(seam.token_before_is(TokenType::OpenBrace)
            || seam.token_before_is(TokenType::CloseParenthesis))
        || !seam.token_after_is_case_or_default()
    {
        return None;
    }

    let target_owner = following_owner.or_else(|| {
        let preceding_owner = preceding_owner?;
        if tree.get_node_type(preceding_owner) != NodeType::Expression {
            return None;
        }

        let expression_id = LocalNodeId::<Expression>::new(preceding_owner);
        let Expression::Match { cases, .. } = tree.get(expression_id) else {
            return None;
        };

        cases.first().map(|case_id| case_id.id)
    })?;
    let target_owner =
        promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::MatchCase)
            .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Resolve trailing comments after switch labels.
fn try_attach_switch_label_trailing_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline
        || !seam.has_trailing_newline
        || !(seam.comment_is_line || seam.comment_is_star)
        || !seam.token_before_is(TokenType::Colon)
        || seam.token_after_is_case_or_default()
    {
        return None;
    }

    let match_case_owner = switch_label_comment_match_case_owner(
        tree,
        parents,
        context,
        preceding_owner,
        following_owner,
    )?;

    // non-default labels keep trailing line comments with the following statement seam
    if !match_case_owner_is_default(tree, match_case_owner) {
        if !seam.comment_is_line {
            return None;
        }

        let target_owner = following_owner
            .or(preceding_owner)
            .map(|owner| normalize_formatter_trivia_target_owner(tree, owner))?;
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    // default line comments before explicit block consequents become block-leading comments
    if seam.comment_is_line
        && let Some(attachment) =
            switch_label_explicit_block_attachment(tree, match_case_owner, seam)
    {
        return Some(attachment);
    }

    let target_owner = normalize_formatter_trivia_target_owner(tree, match_case_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve trailing parameter or argument separator comments before `)`.
fn try_attach_parameter_or_argument_trailing_separator_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline || !seam.has_trailing_newline {
        return None;
    }

    let separator_owner = separator_preceding_owner(tree, context, seam, preceding_owner);
    try_attach_parameter_or_argument_separator_comment(
        tree,
        parents,
        context,
        seam,
        separator_owner,
    )
}

/// Resolve trailing callback argument comments after `,`.
fn try_attach_callback_argument_trailing_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    preceding_owner: Option<u32>,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline
        || !seam.has_trailing_newline
        || !seam.comment_is_line
        || !seam.token_before_is(TokenType::Comma)
        || seam.token_after_is(TokenType::CloseParenthesis)
    {
        return None;
    }

    let target_owner = preceding_owner?;
    promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Argument)?;

    let target_owner = normalize_argument_owner(tree, parents, target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve statement-prefix seam comment rules.
struct StatementPrefixDispatchContext<'a, 'cache> {
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
}

/// One statement-prefix attachment handler in priority order.
type StatementPrefixHandler =
    fn(&mut StatementPrefixDispatchContext<'_, '_>) -> Option<CommentAttachment>;

/// Run ordered statement-prefix attachment handlers.
fn run_statement_prefix_handlers(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
    handlers: &[StatementPrefixHandler],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Attach decorator own-line statement-prefix comments.
fn attach_statement_prefix_decorator_own_line(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_decorator_own_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.following,
    )
}

/// Attach separator own-line statement-prefix comments.
fn attach_statement_prefix_separator_own_line(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_separator_own_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.preceding,
    )
}

/// Attach empty-statement semicolon statement-prefix comments.
fn attach_statement_prefix_empty_statement_semicolon(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_empty_statement_semicolon_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.preceding,
        ctx.owners.following,
    )
}

/// Attach else-body own-line statement-prefix comments.
fn attach_statement_prefix_else_body_own_line(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_else_body_own_line_comment(ctx.tree, ctx.parents, ctx.seam, ctx.owners.following)
}

/// Attach control-head own-line statement-prefix comments.
fn attach_statement_prefix_control_head_own_line(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_control_head_own_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.preceding,
        ctx.owners.following,
    )
}

/// Attach semicolon-guard own-line statement-prefix comments.
fn attach_statement_prefix_semicolon_guard_own_line(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.seam.token_after_is(TokenType::Semicolon) {
        return None;
    }

    attach_semicolon_guard_own_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners,
    )
}

/// Attach switch-label body own-line statement-prefix comments.
fn attach_statement_prefix_switch_label_body_own_line(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_switch_label_body_own_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.preceding,
        ctx.owners.following,
    )
}

/// Attach switch-label next-case own-line statement-prefix comments.
fn attach_statement_prefix_switch_label_next_case_own_line(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_switch_label_next_case_own_line_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
    )
}

/// Attach switch-case prefix own-line statement-prefix comments.
fn attach_statement_prefix_switch_case_prefix_own_line(
    ctx: &mut StatementPrefixDispatchContext<'_, '_>,
) -> Option<CommentAttachment> {
    try_attach_switch_case_prefix_own_line_comment(
        ctx.tree,
        ctx.seam_ctx,
        ctx.seam,
        ctx.enclosing_owner_cache,
        ctx.owners.following,
    )
}

/// Ordered statement-prefix seam handlers.
const STATEMENT_PREFIX_HANDLERS: &[StatementPrefixHandler] = &[
    attach_statement_prefix_decorator_own_line,
    attach_statement_prefix_separator_own_line,
    attach_statement_prefix_empty_statement_semicolon,
    attach_statement_prefix_else_body_own_line,
    attach_statement_prefix_control_head_own_line,
    attach_statement_prefix_semicolon_guard_own_line,
    attach_statement_prefix_switch_label_body_own_line,
    attach_statement_prefix_switch_label_next_case_own_line,
    attach_statement_prefix_switch_case_prefix_own_line,
];

/// Resolve statement-prefix seam comment rules.
pub(crate) fn try_attach_comment_statement_prefix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let mut ctx = StatementPrefixDispatchContext {
        tree,
        parents,
        seam_ctx: context,
        seam,
        enclosing_owner_cache,
        owners,
    };

    run_statement_prefix_handlers(&mut ctx, STATEMENT_PREFIX_HANDLERS)
}

struct StatementSuffixDispatchContext<'a> {
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// The seam context.
    seam_ctx: &'a CommentSeamContext<'a>,
    /// The seam facts.
    seam: &'a CommentSeamData,
    /// Neighbor owner candidates.
    owners: CommentAttachmentNeighbors,
}

/// One statement-suffix attachment handler in priority order.
type StatementSuffixHandler = fn(&StatementSuffixDispatchContext<'_>) -> Option<CommentAttachment>;

/// Run ordered statement-suffix attachment handlers.
fn run_statement_suffix_handlers(
    ctx: &StatementSuffixDispatchContext<'_>,
    handlers: &[StatementSuffixHandler],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Attach else-body inline-line statement-suffix comments.
fn attach_statement_suffix_else_body_inline_line(
    ctx: &StatementSuffixDispatchContext<'_>,
) -> Option<CommentAttachment> {
    try_attach_else_body_inline_line_comment(ctx.tree, ctx.parents, ctx.seam, ctx.owners.following)
}

/// Attach else-body inline-star statement-suffix comments.
fn attach_statement_suffix_else_body_inline_star(
    ctx: &StatementSuffixDispatchContext<'_>,
) -> Option<CommentAttachment> {
    try_attach_else_body_inline_star_comment(ctx.tree, ctx.parents, ctx.seam, ctx.owners.following)
}

/// Attach control-head inline statement-suffix comments.
fn attach_statement_suffix_control_head_inline(
    ctx: &StatementSuffixDispatchContext<'_>,
) -> Option<CommentAttachment> {
    try_attach_control_head_inline_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.preceding,
        ctx.owners.following,
    )
}

/// Attach switch-header trailing statement-suffix comments.
fn attach_statement_suffix_switch_header_trailing(
    ctx: &StatementSuffixDispatchContext<'_>,
) -> Option<CommentAttachment> {
    try_attach_switch_header_trailing_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.owners.preceding,
        ctx.owners.following,
    )
}

/// Attach switch-label trailing statement-suffix comments.
fn attach_statement_suffix_switch_label_trailing(
    ctx: &StatementSuffixDispatchContext<'_>,
) -> Option<CommentAttachment> {
    try_attach_switch_label_trailing_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.preceding,
        ctx.owners.following,
    )
}

/// Attach declaration return-type seam statement-suffix comments.
fn attach_statement_suffix_declaration_return_type(
    ctx: &StatementSuffixDispatchContext<'_>,
) -> Option<CommentAttachment> {
    try_attach_comment_declaration_return_type_seam(ctx.tree, ctx.seam, ctx.owners)
}

/// Attach parameter or argument trailing separator statement-suffix comments.
fn attach_statement_suffix_parameter_or_argument_trailing_separator(
    ctx: &StatementSuffixDispatchContext<'_>,
) -> Option<CommentAttachment> {
    try_attach_parameter_or_argument_trailing_separator_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        ctx.owners.preceding,
    )
}

/// Attach callback argument trailing statement-suffix comments.
fn attach_statement_suffix_callback_argument_trailing(
    ctx: &StatementSuffixDispatchContext<'_>,
) -> Option<CommentAttachment> {
    try_attach_callback_argument_trailing_comment(
        ctx.tree,
        ctx.parents,
        ctx.seam,
        ctx.owners.preceding,
    )
}

/// Ordered statement-suffix seam handlers.
const STATEMENT_SUFFIX_HANDLERS: &[StatementSuffixHandler] = &[
    attach_statement_suffix_else_body_inline_line,
    attach_statement_suffix_else_body_inline_star,
    attach_statement_suffix_control_head_inline,
    attach_statement_suffix_switch_header_trailing,
    attach_statement_suffix_switch_label_trailing,
    attach_statement_suffix_declaration_return_type,
    attach_statement_suffix_parameter_or_argument_trailing_separator,
    attach_statement_suffix_callback_argument_trailing,
];

/// Resolve statement-suffix seam comment rules.
pub(crate) fn try_attach_comment_statement_suffix(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let ctx = StatementSuffixDispatchContext {
        tree,
        parents,
        seam_ctx: context,
        seam,
        owners,
    };

    run_statement_suffix_handlers(&ctx, STATEMENT_SUFFIX_HANDLERS)
}

/// Resolve block body seam comment rules.
pub(crate) fn try_attach_comment_block_body(
    tree: &NodeTree,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let following_owner = owners.following;

    // comments between method signatures and opening braces should stay inside the body
    if !seam.has_leading_newline
        && seam.has_trailing_newline
        && seam.token_after_is(TokenType::OpenBrace)
        && let Some(target_owner) = following_owner
    {
        if tree.get_node_type(target_owner) == NodeType::Block {
            let block_id = LocalNodeId::<Block>::new(target_owner);
            let (target_owner, position) = block_leading_comment_target(tree, block_id);
            return Some((Some(target_owner), position));
        }

        if tree.get_node_type(target_owner) == NodeType::Declaration {
            return Some((Some(target_owner), AnnotationPosition::BlockInfix));
        }

        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::BlockPrefix));
    }

    None
}
