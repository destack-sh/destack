use super::format_generic_argument_list;
use crate::format::annotation::{
    format_trailing_comment_slice, write_raw_leading_comments,
    write_raw_trailing_comments_without_parent_expansion,
};
use crate::format::chain::{member_property_start, transparent_inner_expression};
use crate::format::context::{DestackFormatterCommentExt, ParenthesizedExpressionView};
use crate::format::operator::{needs_parens_in_postfix_position, write_postfix_base_expression};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Comment, Expression, GenericArgument, LocalNodeId, NodeTree, NodeType, PostfixPosition,
    TypeExpression,
};
use destack_core::StringId;
use destack_fir::format::{
    Buffer, FormatError, FormatNode, FormatResult, LineMode, RemoveSoftLinesBuffer, TextWidth,
};
use destack_fir::prelude::{
    format_with, group, indent, line_suffix_boundary, soft_block_indent, soft_line_break, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

#[derive(Clone, Copy)]
enum TypeTemplateSpanLayout {
    SingleLine,
    Fit,
}

#[derive(Clone, Copy, Debug)]
enum StaticMemberLayout {
    NoBreak,
    BreakAfterObject,
}

/// Return whether one member receiver ends with static instantiation arguments.
fn expression_has_trailing_static_instantiation(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Instantiation {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),
        Expression::QualifiedReference {
            generic_arguments, ..
        }
        | Expression::Member {
            generic_arguments, ..
        }
        | Expression::PrivateMember {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),
        Expression::Parenthesized { expression } => {
            expression_has_trailing_static_instantiation(tree, *expression)
        }
        _ => false,
    }
}

/// Return whether one member chain contains any optional chaining segment.
pub(crate) fn member_expression_has_optional_chain(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        match context.tree.get(current_id) {
            Expression::Maybe { .. } => return true,
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Call { left, .. }
            | Expression::Must { left, .. }
            | Expression::Instantiation { left, .. } => current_id = *left,
            Expression::Parenthesized { expression } => {
                current_id = *expression;
            }
            _ => return false,
        }
    }
}

/// Return whether one expression is a function or class declaration expression.
fn expression_is_function_or_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        destack_ast::Declaration::Function(_) | destack_ast::Declaration::Class(_)
    )
}

/// Return whether one parent expression uses a parenthesized object directly as its postfix base.
fn parent_expression_uses_parenthesized_object_directly(
    parenthesized_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
) -> bool {
    match parent_expression {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Must { left, .. }
        | Expression::New { left, .. } => *left == parenthesized_id,
        Expression::Call { left, position, .. } | Expression::Index { left, position, .. } => {
            *left == parenthesized_id && *position == PostfixPosition::Direct
        }
        _ => false,
    }
}

/// Return whether a postfix continuation requires one explicit parenthesized object wrapper.
pub(crate) fn postfix_continuation_requires_parenthesized_object_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if !parent_expression_uses_parenthesized_object_directly(parenthesized_id, parent_expression) {
        return false;
    }

    member_expression_has_optional_chain(context, inner_expression_id)
        || expression_is_function_or_class_declaration(context, inner_expression_id)
}

/// Decide whether a parenthesized expression can be unwrapped in member object position.
pub(crate) fn should_unwrap_parenthesized_member_object(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    // prefix comments and docs on the wrapper itself carry grouping ownership semantics
    if super::expression_has_only_prefix_comment_or_doc_annotations(context, parenthesized_id) {
        return false;
    }

    // object members require explicit grouping: `({}).x`
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::ObjectExpression { .. }
    ) {
        return false;
    }

    // keep nested type slot grouping stable across repeated formatting
    if context.is_in_type_expression_root(parenthesized_id) {
        return false;
    }

    // function and class declarations require grouping before postfix continuations
    if expression_is_function_or_class_declaration(context, inner_expression_id) {
        return false;
    }

    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        // allow unwrapping only when inner annotations are prefix comments or docs
        if !super::expression_has_only_prefix_comment_or_doc_annotations(
            context,
            inner_expression_id,
        ) {
            return false;
        }
    }

    // preserve wrappers with leading line comments on the wrapped object
    let Some(parenthesized_view) =
        ParenthesizedExpressionView::from_node(context, parenthesized_id)
    else {
        return false;
    };

    if parenthesized_view.has_leading_inner_line_comment() {
        return false;
    }

    if parenthesized_view.has_leading_inner_comments()
        && !super::expression_has_only_prefix_comment_or_doc_annotations(
            context,
            inner_expression_id,
        )
    {
        return false;
    }

    // block comments before `)` belong to the original wrapper
    if !parenthesized_view
        .trailing_inner_block_comments()
        .is_empty()
    {
        return false;
    }

    // comments between `)` and the postfix continuation belong to the wrapper
    if !parenthesized_view.postfix_comments().is_empty() {
        return false;
    }

    // optional chains require explicit grouping in non optional member continuations
    if member_expression_has_optional_chain(context, inner_expression_id) {
        return false;
    }

    !needs_parens_in_postfix_position(context.tree, inner_expression_id)
}

/// Format one member receiver, adding wrapper parentheses when static instantiation tails need grouping.
fn format_member_receiver<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    receiver_id: LocalNodeId<Expression>,
    should_wrap_for_static_instantiation: bool,
) -> FormatResult<()> {
    if should_wrap_for_static_instantiation {
        write!(f, [token("(")])?;
        write_postfix_base_expression(f, receiver_id)?;
        write!(f, [token(")")])?;
        return Ok(());
    }

    write_postfix_base_expression(f, receiver_id)
}

/// Return separator-owned comments between one postfix receiver and its continuation.
fn postfix_separator_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let receiver_id = match context.tree.get(node_id) {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. } => *left,
        _ => return Vec::new(),
    };

    context.raw_comments_before_next_non_trivia_token_after_span(context.span(receiver_id))
}

/// Normalize one member receiver by dropping an allowed parenthesized wrapper.
fn normalized_member_receiver(
    context: &DestackFormatContext<'_>,
    receiver_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    if let Expression::Parenthesized { expression } = context.tree.get(receiver_id)
        && should_unwrap_parenthesized_member_object(context, receiver_id, *expression)
    {
        return *expression;
    }

    receiver_id
}

/// Return whether one expression is a member-chain style receiver.
fn expression_is_member_chain_receiver(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::Maybe { .. }
    )
}

/// Return the first non-memberish parent ancestor for one member expression.
fn first_non_memberish_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let mut current_id = node_id;

    loop {
        let (parent_id, parent_type) = context.parent(current_id)?;
        if parent_type != NodeType::Expression {
            return None;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        if matches!(
            context.tree.get(parent_id),
            Expression::Member { .. }
                | Expression::PrivateMember { .. }
                | Expression::Index { .. }
                | Expression::Maybe { .. }
                | Expression::Must { .. }
        ) {
            current_id = parent_id;
            continue;
        }

        return Some(parent_id);
    }
}

/// Select the shared layout for one static member expression.
fn static_member_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    receiver_id: LocalNodeId<Expression>,
) -> StaticMemberLayout {
    let receiver_id = normalized_member_receiver(context, receiver_id);
    if context.comments().has_leading_own_line_comment(
        member_property_start(context, node_id).unwrap_or(context.span(node_id).start),
    ) {
        return StaticMemberLayout::BreakAfterObject;
    }

    let is_member_chain = expression_is_member_chain_receiver(context, receiver_id);
    let mut parent = context.parent(node_id);
    while let Some((parent_id, parent_type)) = parent {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        if matches!(
            context.tree.get(parent_id),
            Expression::Must { .. } | Expression::Maybe { .. }
        ) {
            parent = context.parent(parent_id);
            continue;
        }

        break;
    }

    let is_nested = match parent {
        Some((parent_id, NodeType::Expression)) => {
            let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
            match parent_expression {
                Expression::Assign { left, .. } => {
                    let no_break = matches!(
                        context.tree.get(receiver_id),
                        Expression::Call { arguments, .. } if !arguments.is_empty()
                    );
                    if no_break || is_member_chain {
                        return StaticMemberLayout::NoBreak;
                    }

                    let _ = left;
                    false
                }
                Expression::Member { .. }
                | Expression::PrivateMember { .. }
                | Expression::Index { .. } => true,
                _ => false,
            }
        }
        Some((parent_id, NodeType::Declarator)) => {
            let no_break = matches!(
                context.tree.get(receiver_id),
                Expression::Call { arguments, .. } if !arguments.is_empty()
            );
            if no_break || is_member_chain {
                return StaticMemberLayout::NoBreak;
            }

            let _ = parent_id;
            false
        }
        _ => false,
    };

    if !is_nested && matches!(context.tree.get(receiver_id), Expression::Identifier { .. }) {
        return StaticMemberLayout::NoBreak;
    }

    match first_non_memberish_parent(context, node_id)
        .map(|parent_id| (parent_id, context.tree.get(parent_id)))
    {
        Some((_, Expression::New { left, .. })) if left.id == node_id.id => {
            StaticMemberLayout::NoBreak
        }
        Some((_, Expression::Assign { left, .. })) => {
            if matches!(context.tree.get(*left), Expression::Identifier { .. }) {
                StaticMemberLayout::BreakAfterObject
            } else {
                StaticMemberLayout::NoBreak
            }
        }
        Some((_, _)) => StaticMemberLayout::BreakAfterObject,
        None => StaticMemberLayout::BreakAfterObject,
    }
}

/// Write one static member continuation.
fn write_static_member_continuation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: Option<StringId>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    write!(f, [token("."), name])?;

    if !generic_arguments.is_empty() {
        format_generic_argument_list(f, generic_arguments)?;
    }

    Ok(())
}

/// Write one private member continuation.
fn write_private_member_continuation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: Option<StringId>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    write!(f, [token("."), token("#"), name])?;

    if !generic_arguments.is_empty() {
        format_generic_argument_list(f, generic_arguments)?;
    }

    Ok(())
}

/// Write one static member expression.
fn write_static_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    receiver_id: LocalNodeId<Expression>,
    name: Option<StringId>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    let receiver_id = normalized_member_receiver(f.context(), receiver_id);
    let wraps_static_instantiation =
        expression_has_trailing_static_instantiation(f.context().tree, receiver_id);
    let boundary_comments = postfix_separator_comments(f.context(), node_id);
    let property_start =
        member_property_start(f.context(), node_id).unwrap_or(f.context().span(node_id).start);

    match static_member_layout(f.context(), node_id, receiver_id) {
        StaticMemberLayout::NoBreak => {
            format_member_receiver(f, receiver_id, wraps_static_instantiation)?;
            if !boundary_comments.is_empty() {
                write!(f, [format_trailing_comment_slice(&boundary_comments)])?;
            }

            write_static_member_continuation(f, name, generic_arguments)
        }
        StaticMemberLayout::BreakAfterObject => {
            format_member_receiver(f, receiver_id, wraps_static_instantiation)?;
            if !boundary_comments.is_empty() {
                write!(f, [format_trailing_comment_slice(&boundary_comments)])?;
            }

            write!(
                f,
                [group(&indent(&format_args![
                    soft_line_break(),
                    format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        if f.context()
                            .comments()
                            .has_leading_own_line_comment(property_start)
                        {
                            let leading_comments = {
                                let comments = f.context().comments();
                                comments.comments_before(property_start).to_vec()
                            };
                            write_raw_leading_comments(f, &leading_comments)?;
                            write!(f, [soft_line_break()])?;
                        }

                        Ok(())
                    }),
                    format_with(|f| write_static_member_continuation(f, name, generic_arguments))
                ]))]
            )
        }
    }
}

/// Write one private member expression.
fn write_private_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    receiver_id: LocalNodeId<Expression>,
    name: Option<StringId>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    let receiver_id = normalized_member_receiver(f.context(), receiver_id);
    let wraps_static_instantiation =
        expression_has_trailing_static_instantiation(f.context().tree, receiver_id);
    let boundary_comments = postfix_separator_comments(f.context(), node_id);

    format_member_receiver(f, receiver_id, wraps_static_instantiation)?;
    if !boundary_comments.is_empty() {
        write!(f, [format_trailing_comment_slice(&boundary_comments)])?;
    }

    write_private_member_continuation(f, name, generic_arguments)
}

/// Format a member expression.
pub(crate) fn format_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    match f.context().tree.get(node_id) {
        Expression::Member {
            left,
            name,
            generic_arguments,
        } => write_static_member_expression(f, node_id, *left, *name, generic_arguments)?,
        Expression::PrivateMember {
            left,
            name,
            generic_arguments,
        } => write_private_member_expression(f, node_id, *left, *name, generic_arguments)?,
        _ => {
            return Err(FormatError::SyntaxError {
                message: "unexpected expression kind for member formatter",
            });
        }
    }
    Ok(())
}

/// Format a type template literal expression.
pub(crate) fn format_type_template_literal<'ast>(
    _node_id: LocalNodeId<TypeExpression>,
    strings: &[StringId],
    spans: &[LocalNodeId<TypeExpression>],
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(strings.len(), spans.len().saturating_add(1));

    let root_boundary_line_comments = spans
        .first()
        .copied()
        .map(|first_span_type_id| {
            f.context()
                .raw_type_position_comments_for(first_span_type_id)
                .into_iter()
                .filter(|comment| comment.is_line())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    for (span_index, (span_expression_id, segment)) in spans.iter().zip(string_segments).enumerate()
    {
        let span = f.context().span(*span_expression_id);
        let boundary_line_comments: &[Comment] = if span_index == 0 {
            &root_boundary_line_comments
        } else {
            &[]
        };
        let trailing_boundary_comments = {
            let comments = f.context().comments();
            comments.comments_before_character(span.end, b'}').to_vec()
        };
        let span_has_internal_comments = !f
            .context()
            .comments_in_range(span.start, span.end)
            .is_empty()
            || !f
                .context()
                .raw_prefix_comments_for(*span_expression_id)
                .is_empty();
        let span_has_trailing_boundary_comments = !trailing_boundary_comments.is_empty();
        let span_has_source_newline =
            type_template_span_has_new_line_in_range(f, *span_expression_id);
        let format_span = format_with(|f| write!(f, [*span_expression_id]));
        let interned_span = if span_has_internal_comments {
            None
        } else {
            f.intern_with_comment_snapshot(&format_span)?
        };
        let span_will_break = interned_span
            .as_ref()
            .is_some_and(type_template_span_format_node_will_break);
        let span_layout = if !span_has_internal_comments
            && !span_has_trailing_boundary_comments
            && !span_has_source_newline
            && !span_will_break
        {
            TypeTemplateSpanLayout::SingleLine
        } else {
            TypeTemplateSpanLayout::Fit
        };
        let format_inner = format_with(move |f| {
            match span_layout {
                TypeTemplateSpanLayout::SingleLine => {
                    if let Some(interned_span) = &interned_span {
                        let mut buffer = RemoveSoftLinesBuffer::new(f);
                        buffer.write_node(interned_span.clone());
                    }
                }
                TypeTemplateSpanLayout::Fit => {
                    if let Some(interned_span) = &interned_span {
                        f.write_node(interned_span.clone());
                    } else {
                        write!(f, [*span_expression_id])?;
                    }
                }
            }

            if !boundary_line_comments.is_empty() {
                write_raw_trailing_comments_without_parent_expansion(f, boundary_line_comments)?;
            }

            if !trailing_boundary_comments.is_empty() {
                write_raw_trailing_comments_without_parent_expansion(
                    f,
                    &trailing_boundary_comments,
                )?;
            }

            Ok(())
        });
        write!(
            f,
            [group(&format_args![
                token("${"),
                format_inner,
                line_suffix_boundary(),
                token("}"),
                *segment,
            ])]
        )?;
    }

    write!(f, [token("`")])
}

/// Return whether one type-template interpolation format node must break.
fn type_template_span_format_node_will_break(node: &FormatNode) -> bool {
    match node {
        FormatNode::Line(LineMode::Hard | LineMode::Empty) => true,
        FormatNode::Token { text } => text.contains('\n'),
        FormatNode::Text { width, .. } | FormatNode::FileSlice { width, .. } => {
            matches!(width, TextWidth::Multiline)
        }
        FormatNode::Interned(interned) => interned
            .iter()
            .any(type_template_span_format_node_will_break),
        FormatNode::BestFitting { variants, .. } => variants
            .most_flat()
            .iter()
            .any(type_template_span_format_node_will_break),
        _ => false,
    }
}

/// Return whether one type-template interpolation has source newlines around or inside it.
fn type_template_span_has_new_line_in_range(
    f: &DestackFormatter<'_, '_>,
    span_expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    let span = f.context().span(span_expression_id);

    if source_has_new_line_before(f.context().file.text(), span.start as usize) {
        return true;
    }

    if source_has_new_line_after(f.context().file.text(), span.end as usize) {
        return true;
    }

    f.context().node_has_newline(span_expression_id)
}

/// Return whether raw source has a newline immediately before one position.
fn source_has_new_line_before(source: &str, position: usize) -> bool {
    let bytes = source.as_bytes();
    let mut current_index = position.min(bytes.len());

    while current_index > 0 {
        current_index -= 1;

        match bytes[current_index] {
            b'\n' | b'\r' => return true,
            b' ' | b'\t' => {}
            _ => return false,
        }
    }

    false
}

/// Return whether raw source has a newline immediately after one position.
fn source_has_new_line_after(source: &str, position: usize) -> bool {
    let bytes = source.as_bytes();
    let mut current_index = position.min(bytes.len());

    while let Some(byte) = bytes.get(current_index).copied() {
        match byte {
            b'\n' | b'\r' => return true,
            b' ' | b'\t' => {}
            _ => return false,
        }

        current_index += 1;
    }

    false
}

/// Format an index expression without considering chaining.
pub(crate) fn write_index_access<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    index_id: LocalNodeId<Expression>,
    should_parenthesize: bool,
    force_expand: bool,
) -> FormatResult<()> {
    let format_index_access = format_with(|f| {
        write!(f, [token("[")])?;
        if should_parenthesize {
            write!(
                f,
                [soft_block_indent(&format_args![
                    token("("),
                    index_id,
                    token(")")
                ])]
            )?;
        } else {
            write!(f, [soft_block_indent(&index_id)])?;
        }
        write!(f, [token("]")])
    });

    if force_expand {
        return write!(f, [group(&format_index_access).should_expand(true)]);
    }

    write!(f, [group(&format_index_access)])
}

/// Format an index expression without considering chaining.
#[inline]
pub(crate) fn format_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Index {
        position,
        left,
        index,
    } = f.context().tree.get(node_id)
    {
        let boundary_comments = postfix_separator_comments(f.context(), node_id);

        write_postfix_base_expression(f, *left)?;
        if !boundary_comments.is_empty() {
            write!(f, [format_trailing_comment_slice(&boundary_comments)])?;
        }

        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(index) = index {
            let should_parenthesize = if matches!(
                f.context().tree.get(*index),
                Expression::Parenthesized { .. }
            ) {
                false
            } else {
                let inner_index_id = transparent_inner_expression(f.context(), *index);
                matches!(
                    f.context().tree.get(inner_index_id),
                    Expression::Assign { .. }
                )
            };
            let left_span = f.context().span(*left);
            let index_span = f.context().span(*index);
            let has_break_after_open = left_span.file == index_span.file
                && left_span.end < index_span.start
                && f.context().has_newline(Span::new(
                    left_span.file,
                    left_span.end,
                    index_span.start,
                ));
            if should_parenthesize && !has_break_after_open {
                write!(f, [token("["), token("("), *index, token(")"), token("]")])?;
            } else {
                write_index_access(f, *index, should_parenthesize, has_break_after_open)?;
            }
        } else {
            write!(f, [token("[]")])?;
        }
    } else {
        return Err(FormatError::SyntaxError {
            message: "unexpected expression kind for index formatter",
        });
    }
    Ok(())
}
