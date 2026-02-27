use ast::{AnnotationPosition, NodeParentIndex, NodeTree, NodeType, TokenType};
use destack_ast as ast;
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use super::attachment::FormatterTriviaOwnerIndex;
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentSeamContext, CommentSeamData,
    CommentSeamKeyword,
};
use super::ownership::{
    find_owner_at_or_after_token_with_node_type, find_smallest_owner_enclosing_range,
    find_smallest_owner_enclosing_token, normalize_formatter_trivia_target_owner,
    promote_owner_to_declaration_ancestor, promote_owner_to_node_type_ancestor,
};
use crate::format::directive::directive_for_node;
use crate::format::expression::format_expression;
use crate::{DestackFormatter, FormatNode};

impl<'ast> FormatNode<'ast, ast::Decorator> for ast::Decorator {
    /// Format one decorator annotation.
    fn format_node(
        &self,
        _node_id: ast::LocalNodeId<ast::Decorator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let tree = f.context().tree;
        let needs_parentheses = decorator_needs_parentheses(tree, self.expression);
        let expression_id = self.expression;
        let expression = f.context().tree.get(expression_id);
        let directive = directive_for_node(f.context(), expression_id);

        write!(f, [token("@")])?;
        if needs_parentheses {
            write!(f, [token("(")])?;
        }

        format_expression(f, expression_id, expression, directive)?;

        if needs_parentheses {
            write!(f, [token(")")])?;
        }

        Ok(())
    }
}

/// Return whether a decorator expression requires parentheses.
fn decorator_needs_parentheses(
    tree: &NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    match tree.get(expression_id) {
        ast::Expression::Parenthesized { .. } => false,
        ast::Expression::Path {
            static_arguments, ..
        } => static_arguments.is_some(),
        ast::Expression::Call { left, .. } => !is_identifier_or_static_member_only(tree, *left),
        ast::Expression::Member {
            left,
            static_arguments,
            ..
        } => static_arguments.is_some() || !is_identifier_or_static_member_only(tree, *left),
        _ => true,
    }
}

/// Return whether an expression is an identifier or static-member-only path.
fn is_identifier_or_static_member_only(
    tree: &NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    match tree.get(expression_id) {
        ast::Expression::Path {
            static_arguments, ..
        } => static_arguments.is_none(),
        ast::Expression::Member {
            left,
            static_arguments,
            ..
        } => static_arguments.is_none() && is_identifier_or_static_member_only(tree, *left),
        _ => false,
    }
}

/// Return whether a declaration owner supports inline head comments before `{`.
fn declaration_owner_has_head_before_open_brace(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Declaration {
        return false;
    }

    let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(owner_id);
    matches!(
        tree.get(declaration_id),
        ast::Declaration::Class { .. }
            | ast::Declaration::Struct { .. }
            | ast::Declaration::Enum { .. }
            | ast::Declaration::Interface { .. }
            | ast::Declaration::Extension { .. }
            | ast::Declaration::Namespace { .. }
            | ast::Declaration::Global { .. }
    )
}

/// Return whether one declaration owner is a `new (...) => ...` function signature.
fn declaration_owner_is_new_signature(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Declaration {
        return false;
    }

    let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(owner_id);
    let ast::Declaration::Function { signature, .. } = tree.get(declaration_id) else {
        return false;
    };

    signature.mode == Some(ast::FunctionMode::New)
}

/// Return the type-value expression owner for one type declaration owner.
fn declaration_owner_type_value_expression_owner(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Declaration {
        return None;
    }

    let declaration_id = ast::LocalNodeId::<ast::Declaration>::new(owner_id);
    let ast::Declaration::Type { value, .. } = tree.get(declaration_id) else {
        return None;
    };

    Some(value.id)
}

/// Promote one owner to the nearest member-like owner.
fn promote_owner_to_member_like_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Member).or_else(|| {
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Property)
    })
}

/// Resolve inline block comments around member optional `?` seams.
pub(crate) fn try_attach_comment_declaration_optional_member_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline || seam.has_trailing_newline || !seam.comment_is_star {
        return None;
    }

    if !seam.token_after_is(TokenType::Maybe) && !seam.token_before_is(TokenType::Maybe) {
        return None;
    }

    let enclosing_owner = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        });

    let target_node = enclosing_owner
        .and_then(|owner| promote_owner_to_member_like_ancestor(tree, parents, owner))
        .or_else(|| {
            owners
                .preceding
                .and_then(|owner| promote_owner_to_member_like_ancestor(tree, parents, owner))
        })
        .or_else(|| {
            owners
                .following
                .and_then(|owner| promote_owner_to_member_like_ancestor(tree, parents, owner))
        })?;

    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Resolve inline block comments between `new` and constructor type parameter lists.
pub(crate) fn try_attach_comment_declaration_new_signature_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline
        || seam.has_trailing_newline
        || !seam.comment_is_star
        || !seam.token_after_is(TokenType::OpenParenthesis)
    {
        return None;
    }

    let enclosing_owner = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        });

    let target_node = enclosing_owner
        .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        .or_else(|| {
            owners
                .preceding
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })
        .or_else(|| {
            owners
                .following
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })
        .filter(|owner| declaration_owner_is_new_signature(tree, *owner))?;

    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Resolve declaration `implements` own-line seam comments.
pub(crate) fn try_attach_comment_declaration_implements_seam(
    tree: &NodeTree,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.has_leading_newline {
        return None;
    }

    if !seam.token_before_is_keyword(CommentSeamKeyword::Implements) {
        return None;
    }

    let target_node = owners.preceding?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockPostfix))
}

/// Resolve declaration head comments before heritage keywords.
pub(crate) fn try_attach_comment_declaration_heritage_head_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.has_trailing_newline {
        return None;
    }

    if !seam.token_before_is(TokenType::GreaterThan) {
        return None;
    }

    let token_after_is_heritage_keyword = seam.token_after_is_keyword(CommentSeamKeyword::Extends)
        || seam.token_after_is_keyword(CommentSeamKeyword::Implements);
    if !token_after_is_heritage_keyword {
        return None;
    }

    let declaration_owner = owners
        .preceding
        .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        .or_else(|| {
            owners
                .following
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })?;
    let declaration_owner = normalize_formatter_trivia_target_owner(tree, declaration_owner);
    Some((
        Some(declaration_owner),
        AnnotationPosition::LinePostfixBoundary,
    ))
}

/// Resolve declaration head comments directly before `{`.
pub(crate) fn try_attach_comment_declaration_head_open_brace_seam(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline || !seam.comment_is_star {
        return None;
    }

    if !seam.token_after_is(TokenType::OpenBrace) {
        return None;
    }

    let target_node = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        })
        .filter(|owner| declaration_owner_has_head_before_open_brace(tree, *owner))
        .or_else(|| {
            owners
                .preceding
                .filter(|owner| declaration_owner_has_head_before_open_brace(tree, *owner))
        })?;

    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockPrefix))
}

/// Resolve declaration decorator-adjacent seam comments.
pub(crate) fn try_attach_comment_declaration_decorator_seam(
    tree: &NodeTree,
    _owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.token_after_is(TokenType::At) {
        return None;
    }

    let member_target_from_token = context.token_after.and_then(|token_index| {
        find_owner_at_or_after_token_with_node_type(
            tree,
            _owner_index,
            token_index,
            NodeType::Member,
        )
        .and_then(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
        })
    });
    let member_target = owners.following.and_then(|owner| {
        promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
    });
    let declaration_target = context
        .token_after
        .and_then(|token_index| {
            find_owner_at_or_after_token_with_node_type(
                tree,
                _owner_index,
                token_index,
                NodeType::Declaration,
            )
        })
        .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        .or_else(|| {
            owners
                .following
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })
        .or_else(|| {
            owners
                .preceding
                .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
        })
        .or_else(|| {
            owners
                .following
                .filter(|owner| tree.get_node_type(*owner) == NodeType::Declaration)
        })
        .or_else(|| {
            owners
                .preceding
                .filter(|owner| tree.get_node_type(*owner) == NodeType::Declaration)
        });

    let target_node = member_target_from_token
        .or(member_target)
        .or(declaration_target)?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    if seam.has_leading_newline {
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Resolve declaration comments between parameter lists and arrows.
pub(crate) fn try_attach_comment_declaration_arrow_seam(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> Option<CommentAttachment> {
    if !seam.comment_is_star {
        return None;
    }

    if !matches!(
        seam.token_after_type,
        Some(TokenType::Arrow | TokenType::ArrowWide)
    ) {
        return None;
    }

    let (token_before_span, token_after_span) =
        (context.token_before_span?, context.token_after_span?);
    let target_node = find_smallest_owner_enclosing_range(
        tree,
        token_before_span.span.start,
        token_after_span.span.end,
    )?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Resolve inline export-head block comments.
pub(crate) fn try_attach_comment_declaration_export_inline_block_seam(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline || seam.has_trailing_newline || !seam.comment_is_star {
        return None;
    }

    if !seam.token_before_is_keyword(CommentSeamKeyword::Export) {
        return None;
    }

    let target_node = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        })?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve declaration export-head seam comments.
pub(crate) fn try_attach_comment_declaration_export_seam(
    tree: &NodeTree,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> Option<CommentAttachment> {
    if !seam.has_trailing_newline {
        return None;
    }

    if !seam.token_before_is_keyword(CommentSeamKeyword::Export) {
        return None;
    }

    let target_node = context
        .token_before_span
        .zip(context.token_after_span)
        .and_then(|(before, after)| {
            find_smallest_owner_enclosing_range(tree, before.span.start, after.span.end)
        })?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Resolve declaration return-type seam comments between `:` and the return type.
pub(crate) fn try_attach_comment_declaration_return_type_seam(
    tree: &NodeTree,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if seam.has_leading_newline || !seam.has_trailing_newline || !seam.comment_is_line {
        return None;
    }

    if !seam.token_before_is_return_type_colon {
        return None;
    }

    let target_node = owners.following?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Resolve type-declaration comments between generic heads and `=`.
pub(crate) fn try_attach_comment_declaration_type_value_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if !seam.comment_is_line || !seam.has_leading_newline || !seam.has_trailing_newline {
        return None;
    }

    let is_type_value_assignment_seam =
        seam.token_after_is(TokenType::Assign) || seam.token_before_is(TokenType::Assign);
    if !is_type_value_assignment_seam {
        return None;
    }

    let declaration_owner = if seam.token_before_is(TokenType::Assign) {
        context
            .token_after_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            .or_else(|| {
                owners
                    .following
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            })
            .or_else(|| {
                owners
                    .preceding
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            })
    } else if seam.token_after_is(TokenType::Assign) {
        context
            .token_before_span
            .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span))
            .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            .or_else(|| {
                owners
                    .preceding
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            })
            .or_else(|| {
                owners
                    .following
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            })
    } else {
        owners
            .preceding
            .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            .or_else(|| {
                owners
                    .following
                    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            })
    }?;
    let target_node = declaration_owner_type_value_expression_owner(tree, declaration_owner)?;
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
    let position = if seam.token_before_is(TokenType::Assign) {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };

    Some((Some(target_node), position))
}

/// Resolve declaration seam comment rules.
pub(crate) fn try_attach_comment_declaration(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    if let Some(attachment) =
        try_attach_comment_declaration_optional_member_seam(tree, parents, context, seam, owners)
    {
        return Some(attachment);
    }

    if let Some(attachment) =
        try_attach_comment_declaration_new_signature_seam(tree, parents, context, seam, owners)
    {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_comment_declaration_implements_seam(tree, seam, owners) {
        return Some(attachment);
    }

    if let Some(attachment) =
        try_attach_comment_declaration_heritage_head_seam(tree, parents, seam, owners)
    {
        return Some(attachment);
    }

    if let Some(attachment) =
        try_attach_comment_declaration_head_open_brace_seam(tree, context, seam, owners)
    {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_comment_declaration_decorator_seam(
        tree,
        owner_index,
        parents,
        context,
        seam,
        owners,
    ) {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_comment_declaration_arrow_seam(tree, context, seam) {
        return Some(attachment);
    }

    if let Some(attachment) =
        try_attach_comment_declaration_export_inline_block_seam(tree, context, seam)
    {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_comment_declaration_export_seam(tree, context, seam) {
        return Some(attachment);
    }

    if let Some(attachment) =
        try_attach_comment_declaration_type_value_seam(tree, parents, context, seam, owners)
    {
        return Some(attachment);
    }

    None
}
