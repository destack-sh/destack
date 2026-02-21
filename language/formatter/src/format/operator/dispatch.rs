use super::assign::format_assign_expression;
use super::binary::{format_binary_expression, format_type_binary_expression};
use super::r#new::format_new_expression;
use crate::analysis::scan::first_non_trivia_token_in_span;
use crate::analysis::timing::tags;
use crate::chain::{
    format_call_expression, format_expression_chain, format_index_expression,
    format_instantiation_expression, format_maybe_expression, format_member_expression,
    has_chain_parent, is_expression_chain, needs_parens_in_postfix_position,
};
use crate::expression::{
    Annotation, AnnotationPosition, Argument, DestackFormatContext, DestackFormatter, Expression,
    FormatResult, LocalNodeId, TypeUnaryOperator, UnaryOperator,
    expression_has_leading_prefix_comment, hard_line_break, space, token,
};
use destack_ast::{Comment, CommentStyle, Mutability, PostfixPosition, TokenType};
use destack_fir::format::{Buffer, Format};
use destack_fir::write;

/// Return whether one expression has a line postfix boundary comment annotation.
fn expression_has_line_postfix_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether a call with a multi-segment path callee should be handled as a chain.
fn call_should_route_to_chain_for_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = context.tree.get(node_id) else {
        return false;
    };
    let Expression::Path { path, .. } = context.tree.get(*left) else {
        return false;
    };
    if path.segments.len() <= 1 {
        return false;
    }

    expression_has_line_postfix_boundary_comment(context, node_id)
        || expression_has_line_postfix_boundary_comment(context, *left)
}

/// Return whether one call should bypass chain routing for multiline template arguments.
fn call_prefers_non_chain_for_multiline_template_argument(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call {
        left,
        dynamic_arguments,
        ..
    } = context.tree.get(node_id)
    else {
        return false;
    };
    if dynamic_arguments.len() != 1 {
        return false;
    }

    let argument_id = dynamic_arguments[0];
    let argument_value_id = match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };
    let is_template_literal = matches!(
        context.tree.get(argument_value_id),
        Expression::TemplateExpression { .. }
    );
    if !is_template_literal || !context.node_has_newline(argument_value_id) {
        return false;
    }

    matches!(
        context.tree.get(*left),
        Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::Call { .. }
    )
}

/// Return whether one call should bypass chain routing for parenthesized await member receivers.
fn call_prefers_non_chain_for_parenthesized_await_member_receiver(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = context.tree.get(node_id) else {
        return false;
    };

    let member_receiver_id = match context.tree.get(*left) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => *left,
        _ => return false,
    };

    let Expression::Parenthesized { expression } = context.tree.get(member_receiver_id) else {
        return false;
    };

    matches!(
        context.tree.get(*expression),
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    )
}

/// Collect block infix comment nodes for one type unary expression node.
fn collect_type_unary_infix_comment_facts(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Vec<(CommentStyle, bool, LocalNodeId<Comment>)> {
    let Some(annotation_ids) = context.annotations(node_id) else {
        return Vec::new();
    };

    let mut comments = Vec::new();
    for annotation_id in annotation_ids {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            continue;
        };
        if position != AnnotationPosition::BlockInfix {
            continue;
        }

        let comment = context.tree.get::<destack_ast::Comment>(node);
        let annotation_span = context.annotation_span(annotation_id);
        comments.push((comment.style, context.has_newline(annotation_span), node));
    }

    comments
}

/// Write a type-unary `as <keyword>` suffix with infix comment seams.
fn write_type_unary_as_keyword_with_infix_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    keyword: &'static str,
) -> FormatResult<()> {
    let infix_comments = collect_type_unary_infix_comment_facts(f.context(), node_id);
    if infix_comments.is_empty() {
        write!(f, [space(), token(keyword)])?;
        return Ok(());
    }

    if infix_comments.len() == 1
        && infix_comments[0].0 == CommentStyle::Slash
        && !infix_comments[0].1
    {
        write!(
            f,
            [
                space(),
                infix_comments[0].2,
                hard_line_break(),
                token(keyword)
            ]
        )?;
        return Ok(());
    }

    if infix_comments.len() == 1
        && infix_comments[0].0 == CommentStyle::Star
        && !infix_comments[0].1
    {
        write!(f, [space(), infix_comments[0].2, space(), token(keyword)])?;
        return Ok(());
    }

    write!(f, [hard_line_break()])?;
    for (comment_index, (_, _, comment_id)) in infix_comments.iter().enumerate() {
        if comment_index > 0 {
            write!(f, [hard_line_break()])?;
        }
        write!(f, [*comment_id])?;
    }
    write!(f, [hard_line_break(), token(keyword)])
}

/// Return whether this type-unary node should keep TypeScript angle assertion syntax.
fn type_unary_prefers_angle_assertion_syntax(
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

/// Format operator and chain expression variants.
pub(crate) fn format_operator_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    let tree = f.context().tree;

    match expression {
        // unary
        Expression::Unary { operator, right } => {
            if operator.is_prefix() {
                let right_needs_await_or_yield_grouping = matches!(
                    tree.get(*right),
                    Expression::Await { .. }
                        | Expression::AwaitMaybe { .. }
                        | Expression::Yield { .. }
                );
                let right_has_leading_prefix_comment =
                    expression_has_leading_prefix_comment(f.context(), *right);
                let right_is_parenthesized =
                    matches!(tree.get(*right), Expression::Parenthesized { .. });
                let right_needs_grouping = right_needs_await_or_yield_grouping
                    || (right_has_leading_prefix_comment && !right_is_parenthesized);
                let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);
                if needs_space {
                    if right_needs_grouping {
                        write!(f, [operator, space(), token("("), right, token(")")])?;
                    } else {
                        write!(f, [operator, space(), right])?;
                    }
                } else if right_needs_grouping {
                    write!(f, [operator, token("("), right, token(")")])?;
                } else {
                    write!(f, [operator, right])?;
                }
            } else {
                write!(f, [right, operator])?;
            }
        }

        // type unary
        Expression::TypeUnary { operator, right } => match operator {
            TypeUnaryOperator::Not => {
                write!(f, [operator, right])?;
            }
            TypeUnaryOperator::Must => {
                write!(f, [right, operator])?;
            }
            TypeUnaryOperator::Newtype
            | TypeUnaryOperator::Type
            | TypeUnaryOperator::Readonly
            | TypeUnaryOperator::Typeof
            | TypeUnaryOperator::Keyof => {
                write!(f, [operator, space(), right])?;
            }
            TypeUnaryOperator::AsConst => {
                if type_unary_prefers_angle_assertion_syntax(f.context(), node_id) {
                    write!(f, [token("<const>"), right])?;
                    return Ok(true);
                }

                let right_has_postfix = f.context().has_postfix_annotation(*right);
                write!(f, [right])?;
                if right_has_postfix {
                    write!(f, [token("as")])?;
                } else {
                    write!(f, [token(" as")])?;
                }
                write_type_unary_as_keyword_with_infix_comments(f, node_id, "const")?;
            }
            TypeUnaryOperator::AsComptime => {
                let right_has_postfix = f.context().has_postfix_annotation(*right);
                write!(f, [right])?;
                if right_has_postfix {
                    write!(f, [token("as")])?;
                } else {
                    write!(f, [token(" as")])?;
                }
                write_type_unary_as_keyword_with_infix_comments(f, node_id, "comptime")?;
            }
        },

        // value
        Expression::ValueOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("^")])?;
            if let Some(mutability) = mutability
                && *mutability == Mutability::Immutable
            {
                write!(f, [token("readonly"), space()])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            right.format(f)?;
        }

        // reference
        Expression::ReferenceOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("&")])?;
            if let Some(mutability) = mutability
                && *mutability == Mutability::Immutable
            {
                write!(f, [token("readonly"), space()])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            right.format(f)?;
        }

        // pointer
        Expression::PointerOf { mutability, right } => {
            write!(f, [token("*")])?;
            if let Some(mutability) = mutability
                && *mutability == Mutability::Immutable
            {
                write!(f, [token("readonly"), space()])?;
            }
            right.format(f)?;
        }

        // member
        Expression::Member { .. } | Expression::PrivateMember { .. } => {
            format_member_or_chain_expression(f, node_id)?;
        }

        // index
        Expression::Index { .. } => {
            format_index_or_chain_expression(f, node_id)?;
        }

        // call
        Expression::Call { .. } => {
            format_call_or_chain_expression(f, node_id)?;
        }

        // instantiation
        Expression::Instantiation { .. } => {
            format_instantiation_or_chain_expression(f, node_id)?;
        }

        // new
        Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CALL);
            format_new_expression(f, node_id, *left, static_arguments, dynamic_arguments)?;
        }

        // delete
        Expression::Delete { value } => {
            write!(f, [token("delete"), space(), value])?;
        }

        // maybe
        Expression::Maybe { .. } => {
            format_maybe_or_chain_expression(f, node_id)?;
        }

        // must
        Expression::Must { position, left } => {
            format_must_or_chain_expression(f, node_id, *position, *left)?;
        }

        // binary
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            format_binary_expression(f, node_id, *left, operator, *right)?;
        }

        // type binary
        Expression::TypeBinary {
            left,
            operator,
            right,
        } => {
            format_type_binary_expression(f, node_id, *left, operator, *right)?;
        }

        // assign
        Expression::Assign {
            left,
            operator,
            right,
        } => {
            format_assign_expression(f, node_id, *left, operator, *right)?;
        }

        // debugger
        Expression::Debugger => {
            write!(f, [token("debugger")])?;
        }

        // stub: placeholder for annotation only files
        Expression::Stub => {}

        // error
        Expression::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

/// Format a member expression or route to chain formatting.
fn format_member_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CHAIN);
        format_expression_chain(f, node_id)?;
    } else {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CALL);
        format_member_expression(f, node_id)?;
    }

    Ok(())
}

/// Format an index expression or route to chain formatting.
fn format_index_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CHAIN);
        format_expression_chain(f, node_id)?;
    } else {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CALL);
        format_index_expression(f, node_id)?;
    }

    Ok(())
}

/// Format a call expression or route to chain formatting.
fn format_call_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let should_route_to_chain = (is_expression_chain(f.context().tree, node_id)
        || call_should_route_to_chain_for_boundary_comment(f.context(), node_id))
        && !call_prefers_non_chain_for_multiline_template_argument(f.context(), node_id)
        && !call_prefers_non_chain_for_parenthesized_await_member_receiver(f.context(), node_id);
    if should_route_to_chain {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CHAIN);
        format_expression_chain(f, node_id)?;
    } else {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CALL);
        format_call_expression(f, node_id)?;
    }

    Ok(())
}

/// Format an instantiation expression or route to chain formatting.
fn format_instantiation_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) && has_chain_parent(f.context(), node_id) {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CHAIN);
        format_expression_chain(f, node_id)?;
    } else {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CALL);
        format_instantiation_expression(f, node_id)?;
    }

    Ok(())
}

/// Format a maybe expression or route to chain formatting.
fn format_maybe_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CHAIN);
        format_expression_chain(f, node_id)?;
    } else {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CALL);
        format_maybe_expression(f, node_id)?;
    }

    Ok(())
}

/// Format a must expression or route to chain formatting.
fn format_must_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    position: PostfixPosition,
    left: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if is_expression_chain(f.context().tree, node_id) {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CHAIN);
        format_expression_chain(f, node_id)?;
    } else {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_OPERATOR_CALL);
        let needs_parentheses = needs_parens_in_postfix_position(f.context().tree, left);
        if needs_parentheses {
            write!(f, [token("("), left, token(")")])?;
        } else {
            write!(f, [left])?;
        }
        if position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        write!(f, [token("!")])?;
    }

    Ok(())
}
