use super::{
    format_static_argument_list, is_type_cast_comment_node,
    parenthesized_has_leading_inner_comments, parenthesized_has_leading_inner_line_comment,
    parenthesized_has_leading_inner_trivia,
};
use crate::format::chain::{receiver_is_await_wrapped, transparent_inner_expression};
use crate::format::declaration::expression_is_decorated_class_declaration;
use crate::format::operator::{
    type_union_has_explicit_leading_separator, write_postfix_base_expression,
};
use crate::{DestackFormatter, ExpressionFormatRole};
use destack_ast::{Expression, LocalNodeId, NodeTree, PostfixPosition};
use destack_core::StringId;
use destack_fir::format::{
    Buffer, FormatNode, FormatResult, LineMode, RemoveSoftLinesBuffer, TextWidth,
};
use destack_fir::prelude::{
    format_with, group, indent, line_postfix_boundary, soft_block_indent, soft_line_break, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

#[derive(Clone, Copy)]
enum TypeTemplateSpanLayout {
    SingleLine,
    Fit,
}

/// Return whether one member receiver ends with static instantiation arguments.
fn expression_has_trailing_static_instantiation(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        Expression::QualifiedReference {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Parenthesized { expression } => {
            expression_has_trailing_static_instantiation(tree, *expression)
        }
        _ => false,
    }
}

/// Return whether one member chain contains any optional chaining segment.
pub(crate) fn member_expression_has_optional_chain(
    context: &crate::DestackFormatContext<'_>,
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
    context: &crate::DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        destack_ast::Declaration::Function { .. } | destack_ast::Declaration::Class { .. }
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
    context: &crate::DestackFormatContext<'_>,
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
    context: &crate::DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    // closure style casts bind to the parenthesized wrapper
    if is_type_cast_comment_node(context, parenthesized_id) {
        return false;
    }

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
    if context
        .expression_format_role(parenthesized_id)
        .is_some_and(|role| role == ExpressionFormatRole::Type)
    {
        return false;
    }

    // function and class declarations require grouping before postfix continuations
    if expression_is_function_or_class_declaration(context, inner_expression_id) {
        return false;
    }

    // decorated class expressions require explicit grouping before member access
    if expression_is_decorated_class_declaration(context, inner_expression_id) {
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

    // preserve wrappers with leading line comments to stabilize member object comment seams
    if parenthesized_has_leading_inner_line_comment(context, parenthesized_id, inner_expression_id)
    {
        return false;
    }

    if parenthesized_has_leading_inner_comments(context, parenthesized_id, inner_expression_id)
        && !super::expression_has_only_prefix_comment_or_doc_annotations(
            context,
            inner_expression_id,
        )
    {
        return false;
    }

    // optional chains require explicit grouping in non optional member continuations
    if member_expression_has_optional_chain(context, inner_expression_id) {
        return false;
    }

    !crate::format::operator::needs_parens_in_postfix_position(context.tree, inner_expression_id)
}

/// Return whether a member object should keep parentheses as a `new` callee.
pub(crate) fn member_object_prefers_new_callee_parentheses(
    context: &crate::DestackFormatContext<'_>,
    object_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = object_id;

    while let Expression::Parenthesized { expression } = context.tree.get(current_id) {
        if context.has_annotation(current_id)
            || parenthesized_has_leading_inner_trivia(context, current_id, *expression)
        {
            return false;
        }

        current_id = *expression;
    }

    matches!(
        context.tree.get(current_id),
        Expression::Call { .. } | Expression::Instantiation { .. }
    )
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

/// Format a member expression.
pub(crate) fn format_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    match f.context().tree.get(node_id) {
        Expression::Member {
            left,
            name,
            static_arguments,
        } => {
            let left = if let Expression::Parenthesized { expression } = f.context().tree.get(*left)
                && should_unwrap_parenthesized_member_object(f.context(), *left, *expression)
            {
                *expression
            } else {
                *left
            };
            let is_breakable_member_receiver = matches!(
                f.context().tree.get(left),
                Expression::Call { .. } | Expression::Instantiation { .. }
            ) && !receiver_is_await_wrapped(f.context(), left);
            let should_wrap_receiver_for_static_instantiation =
                expression_has_trailing_static_instantiation(f.context().tree, left);

            if is_breakable_member_receiver {
                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| {
                            format_member_receiver(
                                f,
                                left,
                                should_wrap_receiver_for_static_instantiation,
                            )
                        }),
                        indent(&format_with(|f| {
                            write!(f, [soft_line_break(), token("."), *name])?;
                            if let Some(static_arguments) = static_arguments {
                                format_static_argument_list(f, static_arguments)?;
                            }
                            Ok(())
                        }))
                    ])]
                )?;
            } else {
                format_member_receiver(f, left, should_wrap_receiver_for_static_instantiation)?;
                write!(f, [token(".")])?;
                write!(f, [*name])?;
                if let Some(static_arguments) = static_arguments {
                    format_static_argument_list(f, static_arguments)?;
                }
            }
        }
        Expression::PrivateMember {
            left,
            name,
            static_arguments,
        } => {
            let left = if let Expression::Parenthesized { expression } = f.context().tree.get(*left)
                && should_unwrap_parenthesized_member_object(f.context(), *left, *expression)
            {
                *expression
            } else {
                *left
            };
            let is_breakable_member_receiver = matches!(
                f.context().tree.get(left),
                Expression::Call { .. } | Expression::Instantiation { .. }
            ) && !receiver_is_await_wrapped(f.context(), left);
            let should_wrap_receiver_for_static_instantiation =
                expression_has_trailing_static_instantiation(f.context().tree, left);

            if is_breakable_member_receiver {
                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| {
                            format_member_receiver(
                                f,
                                left,
                                should_wrap_receiver_for_static_instantiation,
                            )
                        }),
                        indent(&format_with(|f| {
                            write!(f, [soft_line_break(), token("."), token("#"), *name])?;
                            if let Some(static_arguments) = static_arguments {
                                format_static_argument_list(f, static_arguments)?;
                            }
                            Ok(())
                        }))
                    ])]
                )?;
            } else {
                format_member_receiver(f, left, should_wrap_receiver_for_static_instantiation)?;
                write!(f, [token("."), token("#"), *name])?;
                if let Some(static_arguments) = static_arguments {
                    format_static_argument_list(f, static_arguments)?;
                }
            }
        }
        _ => {
            debug_assert!(false, "unexpected expression kind for member formatter");
        }
    }
    Ok(())
}

/// Format a type index expression without considering chaining.
#[inline]
pub(crate) fn format_type_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    index: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let needs_parentheses = type_index_left_requires_parentheses(f.context().tree.get(left));
    if needs_parentheses {
        write!(f, [token("("), left, token(")")])?;
    } else {
        write!(f, [left])?;
    }
    write!(f, [token("["), index, token("]")])?;
    Ok(())
}

/// Return whether one type-index left expression requires explicit parentheses.
pub(crate) fn type_index_left_requires_parentheses(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
    )
}

/// Format a type template literal expression.
pub(crate) fn format_type_template_literal<'ast>(
    strings: &[StringId],
    spans: &[LocalNodeId<Expression>],
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(strings.len(), spans.len().saturating_add(1));

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    for (span_expression_id, segment) in spans.iter().zip(string_segments) {
        let span = f.context().span(*span_expression_id);
        let span_has_comments = !f
            .context()
            .comments_in_range(span.start, span.end)
            .is_empty();
        let span_has_source_newline =
            type_template_span_has_new_line_in_range(f, *span_expression_id);
        let format_span = format_with(|f| write!(f, [*span_expression_id]));
        let interned_span = f.intern(&format_span)?;
        let span_will_break = interned_span
            .as_ref()
            .is_some_and(type_template_span_format_node_will_break);
        let span_layout = if span_has_comments || span_has_source_newline || span_will_break {
            TypeTemplateSpanLayout::Fit
        } else {
            TypeTemplateSpanLayout::SingleLine
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
                    }
                }
            }

            Ok(())
        });

        write!(
            f,
            [group(&format_args![
                token("${"),
                format_inner,
                line_postfix_boundary(),
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
    span_expression_id: LocalNodeId<Expression>,
) -> bool {
    let span = f.context().span(span_expression_id);

    if source_has_new_line_before(f.context().file.text(), span.start as usize) {
        return true;
    }

    if source_has_new_line_after(f.context().file.text(), span.end as usize) {
        return true;
    }

    if type_union_has_explicit_leading_separator(f.context(), span_expression_id) {
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
        write_postfix_base_expression(f, *left)?;
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
        debug_assert!(false, "unexpected expression kind for index formatter");
    }
    Ok(())
}
