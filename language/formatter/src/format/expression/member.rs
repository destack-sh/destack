use super::format_generic_argument_list;
use crate::format::annotation::{FormatLeadingComments, FormatTrailingComments};
use crate::format::chain::{member_property_start, transparent_inner_expression};
use crate::format::operator::{assign_pattern_target_expression, write_postfix_base_expression};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Comment, Expression, GenericArgument, LocalNodeId, NodeType, PostfixPosition, ScalarLiteral,
    TypeExpression,
};
use destack_core::StringId;
use destack_fir::format::{
    Buffer, Format, FormatError, FormatNode, FormatNodes, FormatResult, FormatTag,
    RemoveSoftLinesBuffer,
};
use destack_fir::prelude::{
    align, dedent_to_root, format_with, group, indent, line_suffix_boundary, soft_block_indent,
    soft_line_break, token,
};
use destack_fir::{format_args, write};

#[derive(Clone, Copy)]
enum TemplateInterpolationLayout {
    SingleLine,
    Fit,
}

/// Return whether one type-template interpolation spans any surrounding newline trivia.
fn type_template_interpolation_has_newline_in_range(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    let span = context.span(expression_id);
    context.source_text().contains_newline(span)
}

/// Format one type-template interpolation body with separator comments.
fn format_type_template_interpolation_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let span = f.context().span(expression_id);
    let trailing_comments = f
        .context()
        .comments()
        .comments_before_character(span.start, b'}')
        .to_vec();

    write!(f, [expression_id])?;

    if !trailing_comments.is_empty() {
        write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
struct TemplateInterpolationIndentation(u32);

impl TemplateInterpolationIndentation {
    /// Return the indent level part of one template interpolation indentation.
    fn level(self, indent_width: u8) -> u32 {
        self.0 / u32::from(indent_width)
    }

    /// Return the aligned-space remainder of one template interpolation indentation.
    fn align(self, indent_width: u8) -> u8 {
        let remainder = self.0 % u32::from(indent_width);
        remainder.try_into().unwrap_or(u8::MAX)
    }

    /// Compute the indentation after the last newline in one string segment.
    fn after_last_newline(text: &str, indent_width: u8, previous_indentation: Self) -> Self {
        let Some((_, after_newline)) = text.rsplit_once('\n') else {
            return previous_indentation;
        };

        let mut size = 0_u32;
        for byte in after_newline.bytes() {
            match byte {
                b'\t' => {
                    let indent_width = u32::from(indent_width);
                    size = size + indent_width - (size % indent_width);
                }
                b' ' => {
                    size += 1;
                }
                _ => break,
            }
        }

        Self(size)
    }
}

#[derive(Clone, Copy, Debug)]
enum StaticMemberLayout {
    NoBreak,
    BreakAfterObject,
}

/// Format one member receiver.
fn format_member_receiver<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    receiver_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_postfix_base_expression(f, receiver_id)
}

/// Return one receiver after absorbing an optional-chain marker into the member operator.
fn optional_member_receiver(
    context: &DestackFormatContext<'_>,
    receiver_id: LocalNodeId<Expression>,
) -> (LocalNodeId<Expression>, Option<PostfixPosition>) {
    let Expression::Maybe { left, position } = context.tree.get(receiver_id) else {
        return (receiver_id, None);
    };

    (*left, Some(*position))
}

/// Write one static member operator.
fn write_static_member_operator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    optional_position: Option<PostfixPosition>,
) -> FormatResult<()> {
    match optional_position {
        None => write!(f, [token(".")])?,
        Some(PostfixPosition::Direct) => write!(f, [token("?.")])?,
        Some(PostfixPosition::Indirect) => write!(f, [token("."), token("?"), token(".")])?,
    }

    Ok(())
}

/// Return separator comments between one postfix receiver and its continuation.
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

    context.comments_before_next_non_trivia_token_after_span(context.span(receiver_id))
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
            if assign_pattern_target_expression(context, *left).is_some_and(|left_expression_id| {
                matches!(
                    context.tree.get(left_expression_id),
                    Expression::Identifier { .. }
                )
            }) {
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
    optional_position: Option<PostfixPosition>,
    name: Option<StringId>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    write_static_member_operator(f, optional_position)?;
    write!(f, [name])?;

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
    let (receiver_id, optional_position) = optional_member_receiver(f.context(), receiver_id);
    let separator_comments = postfix_separator_comments(f.context(), node_id);
    let property_start =
        member_property_start(f.context(), node_id).unwrap_or(f.context().span(node_id).start);

    match static_member_layout(f.context(), node_id, receiver_id) {
        StaticMemberLayout::NoBreak => {
            format_member_receiver(f, receiver_id)?;
            if !separator_comments.is_empty() {
                write!(f, [FormatTrailingComments::Comments(&separator_comments)])?;
            }

            write_static_member_continuation(f, optional_position, name, generic_arguments)
        }
        StaticMemberLayout::BreakAfterObject => {
            format_member_receiver(f, receiver_id)?;
            if !separator_comments.is_empty() {
                write!(f, [FormatTrailingComments::Comments(&separator_comments)])?;
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
                            write!(f, [FormatLeadingComments::Comments(&leading_comments)])?;
                            write!(f, [soft_line_break()])?;
                        }

                        Ok(())
                    }),
                    format_with(|f| write_static_member_continuation(
                        f,
                        optional_position,
                        name,
                        generic_arguments
                    ))
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
    let separator_comments = postfix_separator_comments(f.context(), node_id);

    format_member_receiver(f, receiver_id)?;
    if !separator_comments.is_empty() {
        write!(f, [FormatTrailingComments::Comments(&separator_comments)])?;
    }

    write_private_member_continuation(f, name, generic_arguments)
}

/// Format a member expression.
pub(crate) fn format_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    match f.context().tree.get(node_id) {
        Expression::Member { left, name } => {
            write_static_member_expression(f, node_id, *left, *name, &[])?
        }
        Expression::PrivateMember { left, name } => {
            write_private_member_expression(f, node_id, *left, *name, &[])?
        }
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

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    let mut indentation = TemplateInterpolationIndentation::default();

    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    for (span_expression_id, segment) in spans.iter().zip(string_segments) {
        let segment_text = f.context().strings.get(*segment);
        indentation = TemplateInterpolationIndentation::after_last_newline(
            segment_text,
            f.options().indent_width,
            indentation,
        );
        let after_newline = segment_text.ends_with('\n');

        let format_span =
            format_with(|f| format_type_template_interpolation_body(f, *span_expression_id));
        let interned_span = f.intern(&format_span)?;
        let span_layout =
            if type_template_interpolation_has_newline_in_range(f.context(), *span_expression_id)
                || interned_span.as_ref().is_some_and(FormatNodes::will_break)
            {
                TemplateInterpolationLayout::Fit
            } else {
                TemplateInterpolationLayout::SingleLine
            };

        let format_inner = format_with(move |f| {
            match span_layout {
                TemplateInterpolationLayout::SingleLine => {
                    if let Some(interned_span) = &interned_span {
                        let mut buffer = RemoveSoftLinesBuffer::new(f);
                        buffer.write_node(interned_span.clone());
                    }
                }
                TemplateInterpolationLayout::Fit => {
                    if let Some(interned_span) = &interned_span {
                        f.write_node(interned_span.clone());
                    }
                }
            }

            Ok(())
        });

        let format_indented = format_with(move |f| {
            if after_newline {
                write!(f, [dedent_to_root(&format_inner)])?;
            } else {
                write_template_interpolation_with_indentation(&format_inner, indentation, f)?;
            }

            Ok(())
        });

        write!(
            f,
            [
                group(&format_args![
                    token("${"),
                    format_indented,
                    line_suffix_boundary(),
                    token("}")
                ]),
                *segment
            ]
        )?;
    }

    write!(f, [token("`")])
}

/// Write one template interpolation with source-derived indentation.
fn write_template_interpolation_with_indentation<'ast>(
    content: &impl Format<DestackFormatContext<'ast>>,
    indentation: TemplateInterpolationIndentation,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    let level = indentation.level(f.options().indent_width);
    let spaces = indentation.align(f.options().indent_width);

    if level == 0 && spaces == 0 {
        write!(f, [content])?;
        return Ok(());
    }

    let format_indented = format_with(|f| {
        for _ in 0..level {
            f.write_node(FormatNode::Tag(FormatTag::StartIndent));
        }

        write!(f, [content])?;

        for _ in 0..level {
            f.write_node(FormatNode::Tag(FormatTag::EndIndent));
        }

        Ok(())
    });

    if spaces == 0 {
        write!(f, [dedent_to_root(&format_indented)])?;
    } else {
        write!(f, [dedent_to_root(&align(spaces, &format_indented))])?;
    }

    Ok(())
}

/// Format an index expression without considering chaining.
pub(crate) fn write_index_access<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    index_id: LocalNodeId<Expression>,
    should_parenthesize: bool,
) -> FormatResult<()> {
    let is_numeric_index = matches!(
        f.context().tree.get(index_id),
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Bigint(_) | ScalarLiteral::Float(_)
        )
    );

    let has_comment_before_close = f
        .context()
        .comments()
        .has_comment_before(f.context().span(node_id).end);
    if is_numeric_index && !should_parenthesize && !has_comment_before_close {
        write!(f, [token("["), index_id, token("]")])?;
        return Ok(());
    }

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
        let separator_comments = postfix_separator_comments(f.context(), node_id);

        write_postfix_base_expression(f, *left)?;
        if !separator_comments.is_empty() {
            write!(f, [FormatTrailingComments::Comments(&separator_comments)])?;
        }

        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(index) = index {
            let inner_index_id = transparent_inner_expression(f.context(), *index);
            let should_parenthesize = matches!(
                f.context().tree.get(inner_index_id),
                Expression::Assign { .. }
            );
            write_index_access(f, node_id, *index, should_parenthesize)?;
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
