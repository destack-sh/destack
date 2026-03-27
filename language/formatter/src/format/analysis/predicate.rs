use crate::format::chain::{argument_value_id_if_present, transparent_inner_expression};
use crate::format::expression::is_trivial_expression;
use crate::format::tree::{argument_is_array_literal, argument_is_object_literal};
use crate::{Annotation, DestackFormatContext};
use destack_ast::{
    AnnotationPosition, Argument, Declaration, Expression, FunctionKind, Keyword, LocalNodeId,
    NodeType, Parameter, TemplateLiteral, TokenSpan, TokenType,
};
use destack_source::Span;

/// Return whether an expression node is a lambda declaration.
fn expression_is_lambda_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function { signature, .. }
                    if signature.kind == FunctionKind::Lambda
            )
    )
}

/// Return whether an argument is trivial and free of annotations or lambda values.
pub(crate) fn argument_is_trivial_unannotated_non_lambda_value(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.has_annotation(argument_id) {
        return false;
    }

    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    if expression_is_lambda_declaration(context, value_id) {
        return false;
    }
    if context.has_annotation(value_id) {
        return false;
    }

    let value = context.tree.get(value_id);
    is_trivial_expression(context.tree, value)
}

/// Return whether an argument is a compact inline callback candidate.
pub(crate) fn argument_is_compact_inline_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);
    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };
    let Declaration::Function {
        signature, body, ..
    } = context.tree.get(*declaration_id)
    else {
        return false;
    };

    if signature.this_parameter.is_some() || signature.return_type.is_some() {
        return false;
    }
    if signature.dynamic_parameters.len() > 1 {
        return false;
    }

    if signature
        .dynamic_parameters
        .first()
        .is_some_and(|parameter_id| {
            !matches!(
                context.tree.get(*parameter_id),
                Parameter::Named {
                    modifiers: None,
                    ty: None,
                    default: None,
                    ..
                }
            )
        })
    {
        return false;
    }

    body.is_some_and(|body_id| {
        let body_id = transparent_inner_expression(context, body_id);

        match context.tree.get(body_id) {
            Expression::Block(block_id) => context.tree.get(*block_id).expressions.len() <= 1,
            Expression::TreeExpression { .. } => true,
            expression => is_trivial_expression(context.tree, expression),
        }
    })
}

/// Return whether an expression appears in call-like argument position.
pub(crate) fn is_call_like_argument(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let Some((expression_id, expression_type)) = context.parent_by_id(argument_id) else {
        return false;
    };
    if expression_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. }
    )
}

/// Check whether an expression is the value of a tree/JSX attribute argument.
pub(crate) fn is_tree_attribute_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let Some((expression_id, expression_type)) = context.parent_by_id(argument_id) else {
        return false;
    };
    if expression_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::TreeExpression { .. }
    )
}

/// Return whether a static argument should stay inline in a path.
pub(crate) fn is_simple_static_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.has_non_blank_annotation(argument_id) {
        return false;
    }

    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value = context.tree.get(value_id);

    is_trivial_expression(context.tree, value)
}

/// Return whether an argument is an interpolated template literal.
pub(crate) fn argument_is_interpolated_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);

    matches!(
        context.tree.get(value_id),
        Expression::TemplateExpression {
            value: TemplateLiteral::InterpolatedString { .. }
        }
    )
}

/// Return whether an argument is a collection literal.
pub(crate) fn argument_is_collection_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_is_object_literal(context, argument_id)
        || argument_is_array_literal(context, argument_id)
}

/// Return whether call arguments span multiple lines in source.
pub(crate) fn call_arguments_are_multiline_span(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first), Some(last)) = (dynamic_arguments.first(), dynamic_arguments.last()) else {
        return false;
    };

    let first_span = context.span(*first);
    let last_span = context.span(*last);
    if first_span.file != last_span.file || first_span.start >= last_span.end {
        return false;
    }

    context.has_newline(Span::new(first_span.file, first_span.start, last_span.end))
}

/// Return whether one token type is ignorable trivia for span-adjacent scans.
#[inline]
fn is_ignored_span_neighbor_token(token_type: TokenType) -> bool {
    matches!(token_type, TokenType::Whitespace | TokenType::Newline)
}

/// Return whether one token type is ignorable trivia, including comments.
#[inline]
fn is_ignored_span_trivia_token(token_type: TokenType) -> bool {
    is_ignored_span_neighbor_token(token_type)
        || matches!(
            token_type,
            TokenType::LineComment
                | TokenType::BlockComment
                | TokenType::DocLineComment
                | TokenType::DocBlockComment
        )
}

/// Return the nearest non-whitespace token before one span.
pub(crate) fn previous_non_whitespace_token_before_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<TokenSpan> {
    context.previous_non_whitespace_token_before_span(span)
}

/// Return the nearest non-whitespace token after one span.
pub(crate) fn next_non_whitespace_token_after_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<TokenSpan> {
    context.next_non_whitespace_token_after_span(span)
}

/// Return the Nth non-trivia token that intersects one span.
pub(crate) fn nth_non_trivia_token_in_span(
    context: &DestackFormatContext<'_>,
    span: Span,
    nth: usize,
) -> Option<TokenSpan> {
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.end <= span.start);
    let mut seen = 0usize;

    while let Some(token) = tokens.get(index).copied() {
        if token.span.start >= span.end {
            break;
        }

        index += 1;
        if is_ignored_span_trivia_token(token.token.ty) {
            continue;
        }

        if seen == nth {
            return Some(token);
        }
        seen += 1;
    }

    None
}

/// Return the first non-trivia token that intersects one span.
#[inline]
pub(crate) fn first_non_trivia_token_in_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<TokenSpan> {
    nth_non_trivia_token_in_span(context, span, 0)
}

/// Return the last non-trivia token that intersects one span.
pub(crate) fn last_non_trivia_token_in_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<TokenSpan> {
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.start < span.end);

    while index > 0 {
        index -= 1;
        let token = tokens[index];
        if token.span.end <= span.start {
            break;
        }
        if is_ignored_span_trivia_token(token.token.ty) {
            continue;
        }

        return Some(token);
    }

    None
}

/// Return the nearest non-whitespace token before one annotation span.
pub(crate) fn previous_non_whitespace_token_before_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<TokenSpan> {
    let span = context.annotation_span(annotation_id);
    previous_non_whitespace_token_before_span(context, span)
}

/// Return the nearest non-whitespace token after one annotation span.
pub(crate) fn next_non_whitespace_token_after_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<TokenSpan> {
    let span = context.annotation_span(annotation_id);
    next_non_whitespace_token_after_span(context, span)
}

/// Return whether one identifier token matches one keyword.
pub(crate) fn token_is_keyword(
    context: &DestackFormatContext<'_>,
    token: TokenSpan,
    keyword: Keyword,
) -> bool {
    context
        .token_keyword(token)
        .is_some_and(|parsed| parsed == keyword)
}

/// Return whether an argument has any slash style comment annotation.
pub(crate) fn argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context.argument_has_line_comment_annotation(argument_id)
}

/// Return whether an argument is an inline closure-cast object argument.
pub(crate) fn argument_is_inline_closure_cast_object(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.node_has_newline(argument_id) {
        return false;
    }

    let argument_span = context.span(argument_id);
    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);
    if !matches!(
        context.tree.get(value_id),
        Expression::ObjectExpression { .. }
    ) {
        return false;
    }

    let value_span = context.span(value_id);
    let argument_has_inline_prefix = context
        .visit_annotations(argument_id, |annotations| {
            if annotations.is_empty() {
                return false;
            }

            annotations
                .iter()
                .all(|annotation_id| match context.annotation(*annotation_id) {
                    Annotation::Blank { .. } => true,
                    Annotation::Doc { position, .. } | Annotation::Comment { position, .. } => {
                        if !matches!(
                            position,
                            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                        ) {
                            return false;
                        }

                        let annotation_span = context.annotation_span(*annotation_id);
                        annotation_span.start < argument_span.start
                    }
                    Annotation::Decorator { .. } => false,
                })
        })
        .unwrap_or(false);
    let value_has_inline_prefix = context
        .visit_annotations(value_id, |annotations| {
            if annotations.is_empty() {
                return false;
            }

            annotations
                .iter()
                .all(|annotation_id| match context.annotation(*annotation_id) {
                    Annotation::Blank { .. } => true,
                    Annotation::Doc { position, .. } | Annotation::Comment { position, .. } => {
                        if !matches!(
                            position,
                            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                        ) {
                            return false;
                        }

                        let annotation_span = context.annotation_span(*annotation_id);
                        annotation_span.start < value_span.start
                    }
                    Annotation::Decorator { .. } => false,
                })
        })
        .unwrap_or(false);

    argument_has_inline_prefix || value_has_inline_prefix
}

/// Return whether a call-like expression has static type arguments.
pub(crate) fn call_has_static_arguments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(node_id) {
        Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        _ => false,
    }
}
