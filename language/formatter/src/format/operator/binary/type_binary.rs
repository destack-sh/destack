use crate::analysis::scan::first_non_trivia_token_in_span;
use crate::call::argument_satisfies_static_seam_comment_annotation_id;
use crate::chain::{is_chain_root, is_expression_chain};
use crate::expression::{
    Annotation, AnnotationPosition, Argument, DestackFormatContext, DestackFormatter, Expression,
    FormatResult, LocalNodeId, ParenthesizedDropPolicy, TokenType, TypeBinaryOperator,
    expression_has_leading_prefix_comment, format_with, group, hard_line_break, indent,
    parenthesized_should_drop, soft_line_break_or_space, space, token,
    type_binary_is_parenthesized_new_callee, type_binary_is_parenthesized_statement_expression,
    type_binary_is_statement_expression,
};
use crate::operator::span_has_comment;
use destack_fir::format::Buffer;
use destack_fir::{format_args, write};

/// Write a cast or satisfies operator and right operand.
fn write_type_binary_operator_and_right<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [operator, space()])?;
    write!(f, [right])
}

/// Return whether this cast expression should keep TypeScript angle assertion syntax.
fn cast_prefers_angle_assertion_syntax(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if context.options.language_type.supports_jsx() {
        return false;
    }

    let Some(main_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    first_non_trivia_token_in_span(context, main_span)
        .is_some_and(|token| token.token.ty == TokenType::LessThan)
}

/// Return one satisfies seam line comment node from rhs ownership variants.
fn satisfies_seam_comment_node_id(
    context: &DestackFormatContext<'_>,
    right_expression_id: LocalNodeId<Expression>,
    static_arguments: &[LocalNodeId<Argument>],
) -> Option<LocalNodeId<destack_ast::Comment>> {
    if let Some(annotation_ids) = context.annotations(right_expression_id) {
        for annotation_id in annotation_ids {
            let Annotation::Comment {
                node,
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
            } = context.annotation(annotation_id)
            else {
                continue;
            };

            let comment = context.tree.get::<destack_ast::Comment>(node);
            if comment.style != destack_ast::CommentStyle::Slash {
                continue;
            }

            return Some(node);
        }
    }

    static_arguments.first().and_then(|argument_id| {
        let annotation_id =
            argument_satisfies_static_seam_comment_annotation_id(context, *argument_id)?;
        let Annotation::Comment { node, .. } = context.annotation(annotation_id) else {
            return None;
        };
        let comment = context.tree.get::<destack_ast::Comment>(node);
        if comment.style != destack_ast::CommentStyle::Slash {
            return None;
        }
        Some(node)
    })
}

/// Try to write one trivial object literal inline for satisfies seam comment layout.
fn try_write_inline_object_left_for_satisfies_seam_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let Expression::ObjectExpression { properties, .. } = f.context().tree.get(expression_id)
    else {
        return Ok(false);
    };
    if properties.len() != 1 {
        return Ok(false);
    }
    if f.context().has_annotation(expression_id) || f.context().node_has_newline(expression_id) {
        return Ok(false);
    }

    let property_id = properties[0];
    if f.context().has_annotation(property_id)
        || f.context().node_has_newline(property_id)
        || span_has_comment(f.context(), f.context().span(property_id))
    {
        return Ok(false);
    }

    write!(f, [token("{"), space(), property_id, space(), token("}")])?;
    Ok(true)
}

/// Format a type-binary expression with chain-aware left-hand expansion.
pub(in crate::format::operator) fn format_type_binary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    operator: &TypeBinaryOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let cast_uses_angle_assertion = *operator == TypeBinaryOperator::Cast
        && cast_prefers_angle_assertion_syntax(f.context(), node_id);

    let mut formatted_left = left;
    if !cast_uses_angle_assertion
        && matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        )
        && let Expression::Parenthesized { expression } = f.context().tree.get(left)
        && parenthesized_should_drop(
            f.context(),
            left,
            *expression,
            ParenthesizedDropPolicy::TypeBinaryLeft { node_id },
        )
    {
        formatted_left = *expression;
    }

    // statement-level satisfies/cast over object literals should keep `({ ... })` lhs wrapping
    let left_needs_statement_object_parentheses =
        matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        ) && matches!(
            f.context().tree.get(formatted_left),
            Expression::ObjectExpression { .. }
        ) && (type_binary_is_statement_expression(f.context(), node_id)
            || type_binary_is_parenthesized_statement_expression(f.context(), node_id));

    let format_left = |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
        if left_needs_statement_object_parentheses {
            write!(f, [token("("), formatted_left, token(")")])
        } else {
            write!(f, [formatted_left])
        }
    };

    if cast_uses_angle_assertion {
        write!(f, [token("<"), right, token(">"), format_with(format_left)])?;
        return Ok(());
    }

    let has_postfix = f.context().has_postfix_annotation(formatted_left);
    let left_has_leading_prefix_comment =
        expression_has_leading_prefix_comment(f.context(), formatted_left);
    let left_is_chain_expression = is_expression_chain(f.context().tree, formatted_left)
        || is_chain_root(f.context().tree, formatted_left);
    let is_parenthesized_new_callee = type_binary_is_parenthesized_new_callee(f.context(), node_id);

    // satisfies separator seam comments before multi-argument static type lists:
    // `... satisfies // note\nRecord<A, B>` -> `... satisfies Record< // note\n    A,\n    B\n>`
    if *operator == TypeBinaryOperator::Satisfies
        && let Expression::Path {
            path,
            static_arguments: Some(static_arguments),
        } = f.context().tree.get(right)
        && static_arguments.len() > 1
        && path.segments.len() == 1
        && let Some(seam_comment_id) =
            satisfies_seam_comment_node_id(f.context(), right, static_arguments)
    {
        let wrote_inline_object_left =
            try_write_inline_object_left_for_satisfies_seam_comment(f, formatted_left)?;
        if !wrote_inline_object_left {
            write!(f, [group(&format_with(format_left))])?;
        }
        if !has_postfix {
            write!(f, [space()])?;
        }
        write!(
            f,
            [
                operator,
                space(),
                path.segments[0],
                token("<"),
                space(),
                seam_comment_id
            ]
        )?;
        write!(
            f,
            [indent(&format_with(|f| {
                write!(f, [hard_line_break()])?;
                for (index, argument_id) in static_arguments.iter().enumerate() {
                    write!(f, [*argument_id])?;
                    if index + 1 < static_arguments.len() {
                        write!(f, [token(","), hard_line_break()])?;
                    }
                }
                Ok(())
            }))]
        )?;
        write!(f, [hard_line_break(), token(">")])?;
        return Ok(());
    }

    let should_expand_chain_left = matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) && left_is_chain_expression
        && (f.context().node_has_newline(node_id)
            || f.context().node_has_newline(formatted_left)
            || is_parenthesized_new_callee);

    if should_expand_chain_left {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        format_left(f)
                    }))
                    .should_expand(true)]
                )?;
                if !has_postfix {
                    write!(f, [space()])?;
                }
                write_type_binary_operator_and_right(f, operator, right)
            }))]
        )?;
    } else {
        let keep_left_and_operator_on_same_line = matches!(
            operator,
            TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
        );

        write!(
            f,
            [group(&format_args![
                format_with(format_left),
                indent(&format_with(|f| {
                    if !has_postfix {
                        if left_has_leading_prefix_comment || keep_left_and_operator_on_same_line {
                            write!(f, [space()])?;
                        } else {
                            write!(f, [soft_line_break_or_space()])?;
                        }
                    }
                    write_type_binary_operator_and_right(f, operator, right)
                }))
            ])]
        )?;
    }

    Ok(())
}
