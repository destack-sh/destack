use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, Declaration, DestackFormatContext, Expression,
    FunctionKind, IfCondition, IfKind, LocalNodeId, ScalarLiteral, Span, collect_chain_nodes,
    has_comment_between_expressions, member_has_intervening_comment, span_has_comment,
    transparent_inner_expression,
};
use crate::format::tree::argument::{
    argument_lambda_declaration_id, argument_transparent_value_id, declaration_is_lambda,
    expression_function_declaration_id, expression_postfix_receiver_id, lambda_body_expression_id,
};

/// Check whether a tree text child is whitespace-only.
pub(crate) fn tree_text_is_whitespace_only(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    match tree.get(*value) {
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            let content = strings.get(*string_id);
            let has_non_whitespace = content.chars().any(|c| !c.is_whitespace());
            if has_non_whitespace {
                return Some((false, false));
            }

            let has_newline = content.contains(['\n', '\r']);
            Some((true, has_newline))
        }
        Expression::ScalarLiteral(ScalarLiteral::Character(value)) => {
            if !value.is_whitespace() {
                return Some((false, false));
            }

            let has_newline = matches!(value, '\n' | '\r');
            Some((true, has_newline))
        }
        _ => None,
    }
}

/// Check whether a tree text child needs separator spaces for newline boundaries.
pub(crate) fn tree_text_boundary_separator_space(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;

    // locate the raw text content
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = tree.get(*value) else {
        return None;
    };

    let text = strings.get(*string_id);

    // identify the boundary whitespace runs
    let leading_end = text
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(text.len(), |(index, _)| index);
    let trailing_start = text
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(0, |(index, c)| index + c.len_utf8());

    let leading_whitespace = &text[..leading_end];
    let trailing_whitespace = &text[trailing_start..];

    let has_leading_whitespace = !leading_whitespace.is_empty();
    let has_trailing_whitespace = !trailing_whitespace.is_empty();

    let leading_is_inline = has_leading_whitespace && !leading_whitespace.contains(['\n', '\r']);
    let trailing_is_inline = has_trailing_whitespace && !trailing_whitespace.contains(['\n', '\r']);

    let needs_leading_separator = has_leading_whitespace && !leading_is_inline;
    let needs_trailing_separator = has_trailing_whitespace && !trailing_is_inline;

    Some((needs_leading_separator, needs_trailing_separator))
}

/// Return whether source preserves an empty line between two tree child arguments.
pub(crate) fn tree_children_have_blank_line_between(
    context: &DestackFormatContext<'_>,
    previous_argument_id: LocalNodeId<Argument>,
    next_argument_id: LocalNodeId<Argument>,
) -> bool {
    let previous_span = context.span(previous_argument_id);
    let next_span = context.span(next_argument_id);
    if previous_span.file != next_span.file {
        return false;
    }
    if previous_span.end >= next_span.start {
        return false;
    }

    let between_span = Span::new(previous_span.file, previous_span.end, next_span.start);
    context.has_blank_line(between_span)
}

/// Check whether a tree child expression should stay inline inside `{ ... }`.
pub(crate) fn tree_child_should_inline_braced_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let value_expr = context.tree.get(value_id);
    let argument_span = context.span(argument_id);
    let value_span = context.span(value_id);

    if argument_span.file == value_span.file {
        if argument_span.start < value_span.start
            && span_has_comment(
                context,
                Span::new(argument_span.file, argument_span.start, value_span.start),
            )
        {
            return false;
        }

        if value_span.end < argument_span.end
            && span_has_comment(
                context,
                Span::new(argument_span.file, value_span.end, argument_span.end),
            )
        {
            return false;
        }
    }

    if argument_has_line_comment_annotation(context, argument_id) {
        return false;
    }

    match value_expr {
        Expression::ScalarLiteral(ScalarLiteral::String(_))
        | Expression::ScalarLiteral(ScalarLiteral::Character(_)) => true,
        Expression::ArrayExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::Call { .. }
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Binary { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => {
            !expression_has_line_comment_annotation(context, value_id)
                && !expression_has_chain_seam_comment(context, value_id)
        }
        Expression::If {
            kind: IfKind::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } => {
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => return false,
            };
            if expression_has_line_comment_annotation(context, value_id)
                || expression_has_line_comment_annotation(context, condition_id)
                || expression_has_line_comment_annotation(context, *then_expression)
                || else_expression
                    .is_some_and(|else_id| expression_has_line_comment_annotation(context, else_id))
            {
                return false;
            }
            true
        }
        Expression::Declaration(declaration_id) => matches!(
            context.tree.get(*declaration_id),
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        ),
        _ => false,
    }
}

/// Return whether one expression chain has comments on member or operator seams.
pub(crate) fn expression_has_chain_seam_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let chain = collect_chain_nodes(context.tree, expression_id);
    if chain.len() <= 1 {
        return false;
    }

    if chain
        .windows(2)
        .any(|adjacent| has_comment_between_expressions(context, adjacent[0], adjacent[1]))
    {
        return true;
    }

    chain
        .iter()
        .copied()
        .any(|chain_node_id| member_has_intervening_comment(context, chain_node_id))
}

/// Return whether one expression has a line-oriented slash comment annotation.
fn expression_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return false;
    };

    annotation_ids.into_iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            return false;
        };

        let is_line_position = matches!(
            position,
            AnnotationPosition::LinePrefix
                | AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
        );
        if !is_line_position {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Return whether one tree argument has a line-oriented slash comment annotation.
fn argument_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotation_ids) = context.annotations(argument_id) else {
        return false;
    };

    annotation_ids.into_iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            return false;
        };

        let is_line_position = matches!(
            position,
            AnnotationPosition::LinePrefix
                | AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
        );
        if !is_line_position {
            return false;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Return whether one ternary expression has any line-comment annotations on its branches.
fn ternary_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::If {
        kind: IfKind::Ternary,
        condition,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(expression_id)
    else {
        return false;
    };

    let condition_id = match condition {
        IfCondition::Expression { condition } => *condition,
        IfCondition::Let { .. } => return true,
    };

    expression_has_line_comment_annotation(context, expression_id)
        || expression_has_line_comment_annotation(context, condition_id)
        || expression_has_line_comment_annotation(context, *then_expression)
        || else_expression
            .is_some_and(|else_id| expression_has_line_comment_annotation(context, else_id))
}

/// Check whether a tree child forces the element to break.
pub(crate) fn tree_child_breaks_element(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let value_expr = context.tree.get(value_id);
    let is_text_node = matches!(
        value_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    );
    let has_line_comment_annotation = argument_has_line_comment_annotation(context, argument_id)
        || expression_has_line_comment_annotation(context, value_id);
    if has_line_comment_annotation {
        return true;
    }

    if (context.has_annotation(argument_id) || context.has_annotation(value_id))
        && !is_text_node
        && !matches!(value_expr, Expression::Stub)
    {
        if matches!(
            value_expr,
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        ) {
            return ternary_has_line_comment_annotation(context, value_id);
        }

        return true;
    }

    match value_expr {
        Expression::Stub => context.options.language_type.is_destack(),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => ternary_has_line_comment_annotation(context, value_id),
        Expression::Block(_) | Expression::Match { .. } => true,
        Expression::Declaration(declaration_id) => {
            lambda_body_is_complex_for_tree(context, *declaration_id)
        }
        Expression::TreeExpression { .. } => false,
        _ => expression_has_complex_callback(context, value_id),
    }
}

/// Check whether a lambda body is complex enough to force tree breaking.
pub(crate) fn lambda_body_is_complex_for_tree(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Some(body_id) = lambda_body_expression_id(context, declaration_id) else {
        return false;
    };

    matches!(
        context.tree.get(body_id),
        Expression::Block(_) | Expression::TreeExpression { .. }
    )
}

/// Check whether an argument is a lambda with a complex body for tree literals.
pub(crate) fn argument_is_complex_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(declaration_id) = argument_lambda_declaration_id(context, argument_id) else {
        return false;
    };

    lambda_body_is_complex_for_tree(context, declaration_id)
}

/// Check whether an argument is a lambda with a block body.
pub(crate) fn argument_is_block_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(declaration_id) = argument_lambda_declaration_id(context, argument_id) else {
        return false;
    };

    lambda_body_expression_id(context, declaration_id)
        .is_some_and(|body_id| matches!(context.tree.get(body_id), Expression::Block(_)))
}

/// Check whether an argument is an object literal expression.
pub(crate) fn argument_is_object_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::ObjectExpression { .. }
        )
    })
}

/// Check whether an argument is an array literal expression.
pub(crate) fn argument_is_array_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::ArrayExpression { .. }
        )
    })
}

/// Check whether an argument is a template literal expression.
pub(crate) fn argument_is_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_transparent_value_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::TemplateExpression { .. }
        )
    })
}

/// Check whether an argument is a lambda expression.
pub(crate) fn argument_is_lambda_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_lambda_declaration_id(context, argument_id).is_some()
}

/// Check whether an argument is a function expression.
pub(crate) fn argument_is_function_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_transparent_value_id(context, argument_id) else {
        return false;
    };
    let Some(declaration_id) = expression_function_declaration_id(context.tree, value_id) else {
        return false;
    };

    !declaration_is_lambda(context.tree, declaration_id)
}

/// Check whether an expression contains a call with a complex callback.
pub(crate) fn expression_has_complex_callback(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    // unwrap transparent wrappers
    let expression_id = transparent_inner_expression(context, expression_id);

    // walk the expression shape looking for callback lambdas
    match tree.get(expression_id) {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => {
            // check the call arguments
            let has_complex_argument = dynamic_arguments
                .iter()
                .any(|arg_id| argument_is_complex_callback(context, *arg_id));

            if has_complex_argument {
                return true;
            }

            // check chained receivers
            expression_has_complex_callback(context, *left)
        }
        expression if let Some(left) = expression_postfix_receiver_id(expression) => {
            expression_has_complex_callback(context, left)
        }
        Expression::Declaration(declaration_id) => {
            lambda_body_is_complex_for_tree(context, *declaration_id)
        }
        Expression::Block(block_id) => {
            let block = tree.get(*block_id);

            // scan block expressions for complex callbacks
            block
                .expressions
                .iter()
                .any(|expr_id| expression_has_complex_callback(context, *expr_id))
        }
        _ => false,
    }
}
