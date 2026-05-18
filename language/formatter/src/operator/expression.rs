use crate::annotation::{infix_or_postfix_annotations, postfix_annotations};
use crate::call::{
    call_should_route_to_chain, expression_is_long_curried_call, format_call_expression,
    format_instantiation_expression, format_new_expression,
};
use crate::chain::{
    chain_has_call_like_expression, format_expression_chain, format_maybe_expression,
    transparent_inner_expression,
};
use crate::declaration::statement::format_block_wide;
use crate::expression::{
    ExpressionLeftSide, expression_needs_parentheses_in_parent, format_index_expression,
    format_member_expression,
};
use crate::operator::assign::format_assign_expression;
use crate::operator::binary::format_binary_expression;
use crate::operator::r#type::{
    format_as_expression, format_is_expression, format_satisfies_expression,
};
use crate::operator::write_postfix_base_expression;
use crate::{DestackFormatContext, DestackFormatter};
use destack_dir::{
    Expression, LocalNodeId, Mutability, NodeType, PostfixPosition, RangeEnd, UnaryOperator,
};
use destack_fir::format::{Buffer, FormatError, FormatResult};
use destack_fir::prelude::{format_with, group, soft_block_indent, space, token};
use destack_fir::write;

/// Return whether one expression is the callee or object for an await grouping context.
fn expression_is_await_callee_or_object_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            *left == node_id
        }
        Expression::Index { left, .. } => *left == node_id,
        Expression::Call { left, .. } | Expression::New { left, .. } => *left == node_id,
        _ => false,
    }
}

/// Return the argument for one await-like expression.
fn await_expression_argument(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match context.tree.get(node_id) {
        Expression::Await { expression }
        | Expression::AwaitMaybe { expression }
        | Expression::AwaitMust { expression } => Some(*expression),
        _ => None,
    }
}

/// Return the nearest await-like expression ancestor.
fn await_expression_ancestor(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let mut current_id = node_id.id;

    loop {
        let (parent_id, parent_type) = context.parent_by_id(current_id)?;
        if parent_type != NodeType::Expression {
            return None;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        if await_expression_argument(context, parent_expression_id).is_some() {
            return Some(parent_expression_id);
        }

        current_id = parent_id;
    }
}

/// Return the leftmost expression reachable from one expression.
fn expression_leftmost(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut leftmost = ExpressionLeftSide::new(node_id);

    while let Some(next_leftmost) = leftmost.left(context) {
        leftmost = next_leftmost;
    }

    leftmost.expression_id()
}

/// Return whether one await-like expression needs grouped object indentation.
fn await_expression_groups_object_indent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(ancestor_id) = await_expression_ancestor(context, node_id) else {
        return true;
    };

    if expression_needs_parentheses_in_parent(context, ancestor_id) {
        return false;
    }

    let Some(argument_id) = await_expression_argument(context, ancestor_id) else {
        return false;
    };

    expression_leftmost(context, argument_id) != node_id
}

/// Write one await-like expression.
fn format_await_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    operand: LocalNodeId<Expression>,
    marker: Option<&'static str>,
) -> FormatResult<()> {
    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("await")])?;
        if let Some(marker) = marker {
            write!(f, [token(marker)])?;
        }
        write!(f, [space(), operand])
    });

    // callee or object indentation
    if expression_is_await_callee_or_object_context(f.context(), node_id) {
        let indented = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [soft_block_indent(&format_inner)])
        });

        if await_expression_groups_object_indent(f.context(), node_id) {
            write!(f, [group(&indented)])?;
        } else {
            write!(f, [indented])?;
        }

        return Ok(());
    }

    write!(f, [format_inner])
}

/// Return whether one operator expression serializes infix annotations as postfix-only annotations.
pub(crate) fn operator_expression_uses_postfix_only_annotations(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    matches!(
        expression,
        Expression::Call {
            arguments,
            ..
        }
        | Expression::New {
            arguments,
            ..
        } if arguments.is_empty() && context.has_infix_annotation(node_id)
    )
}

/// Write trailing annotations for one operator expression.
pub(crate) fn write_operator_expression_trailing_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    // postfix-style operator expressions own their infix serialization locally
    if operator_expression_uses_postfix_only_annotations(f.context(), expression_id, expression) {
        write!(f, [postfix_annotations(f.context(), expression_id)])?;
        return Ok(());
    }

    write!(
        f,
        [infix_or_postfix_annotations(f.context(), expression_id)]
    )
}

/// Return whether one prefix expression operand needs grouping.
fn prefix_expression_operand_needs_grouping(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let right_expression_id = transparent_inner_expression(context, right);
    let right_span = context.span(right_expression_id);
    let right_start = context.expression_token_start(right_expression_id);
    let prefix_span = context.span(node_id);
    let comments = context.comments();

    comments.has_comment_before(right_start)
        || comments.has_comment_in_range(right_span.end, prefix_span.end)
}

/// Write one prefix expression operand with grouped boundary comments.
fn write_prefix_expression_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if prefix_expression_operand_needs_grouping(f.context(), node_id, right) {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [token("("), soft_block_indent(&right), token(")")])
            }))]
        )?;
    } else {
        write!(f, [right])?;
    }

    Ok(())
}

/// Write one range expression.
fn format_range_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    start: Option<LocalNodeId<Expression>>,
    end: Option<LocalNodeId<Expression>>,
    end_kind: RangeEnd,
) -> FormatResult<()> {
    // start bound
    if let Some(start) = start {
        write!(f, [start])?;
    }

    // range operator
    let token_value = match end_kind {
        RangeEnd::Open => "..",
        RangeEnd::Inclusive => "..=",
    };
    write!(f, [token(token_value)])?;

    // end bound
    if let Some(end) = end {
        write!(f, [end])?;
    }

    Ok(())
}

/// Format operator and chain expression variants.
pub(crate) fn format_operator_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    match expression {
        // unary
        Expression::Unary { operator, right } => {
            if operator.is_prefix() {
                let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);

                if needs_space {
                    write!(f, [operator, space()])?;
                } else {
                    write!(f, [operator])?;
                }

                write_prefix_expression_operand(f, node_id, *right)?;
            } else {
                write!(f, [right, operator])?;
            }
        }

        // type expression
        Expression::Type { value } => {
            write!(f, [value])?;
        }

        // value
        Expression::MoveOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("^")])?;
            if let Some(mutability) = mutability {
                match mutability {
                    Mutability::Immutable => write!(f, [token("readonly"), space()])?,
                    Mutability::Exclusive => write!(f, [token("exclusive"), space()])?,
                    Mutability::Mutable => {}
                }
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            write_prefix_expression_operand(f, node_id, *right)?;
        }

        // await-like
        Expression::Await { expression } => {
            format_await_expression(f, node_id, *expression, None)?;
        }
        Expression::AwaitMaybe { expression } => {
            format_await_expression(f, node_id, *expression, Some("?"))?;
        }
        Expression::AwaitMust { expression } => {
            format_await_expression(f, node_id, *expression, Some("!"))?;
        }

        // comptime
        Expression::Comptime { body } => {
            write!(f, [token("comptime"), space()])?;

            if let Expression::Block(block_id) = f.context().tree.get(*body) {
                format_block_wide(f, *block_id)?;
            } else {
                write!(f, [body])?;
            }
        }

        // reference
        Expression::BorrowOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("&")])?;
            if let Some(mutability) = mutability {
                match mutability {
                    Mutability::Immutable => write!(f, [token("readonly"), space()])?,
                    Mutability::Exclusive => write!(f, [token("exclusive"), space()])?,
                    Mutability::Mutable => {}
                }
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            write_prefix_expression_operand(f, node_id, *right)?;
        }

        // member
        Expression::Member { .. } | Expression::PrivateMember { .. } => {
            format_member_expression(f, node_id)?;
        }

        // index
        Expression::Index { left, .. } => {
            if postfix_expression_should_route_to_chain(f.context(), *left) {
                format_expression_chain(f, node_id)?;
            } else {
                format_index_expression(f, node_id)?;
            }
        }

        // call
        Expression::Call { .. } => {
            format_call_or_chain_expression(f, node_id)?;
        }

        // instantiation
        Expression::Instantiation { .. } => {
            format_instantiation_expression(f, node_id)?;
        }

        // new
        Expression::New {
            left,
            generic_arguments,
            arguments,
        } => {
            format_new_expression(f, node_id, *left, generic_arguments, arguments)?;
        }

        // maybe
        Expression::Maybe { .. } => {
            format_maybe_expression(f, node_id)?;
        }

        // must
        Expression::Must { position, left } => {
            format_must_expression(f, *position, *left)?;
        }

        // binary
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            format_binary_expression(f, node_id, *left, operator, *right)?;
        }

        // range
        Expression::RangeExpression {
            start,
            end,
            end_kind,
        } => {
            format_range_expression(f, *start, *end, *end_kind)?;
        }

        // assertions
        Expression::As {
            expression,
            target_type,
        } => {
            format_as_expression(f, node_id, *expression, *target_type)?;
        }
        Expression::Satisfies {
            expression,
            target_type,
        } => {
            format_satisfies_expression(f, node_id, *expression, *target_type)?;
        }
        Expression::Is { value, target_type } => {
            format_is_expression(f, node_id, *value, *target_type)?;
        }
        Expression::InstanceOf { value, target } => {
            write!(f, [value, space(), token("instanceof"), space(), target])?;
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

        // missing: preserve the surrounding source hole
        Expression::Missing => {}

        // error
        Expression::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

/// Format a call expression or route to chain formatting.
fn format_call_or_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let Expression::Call {
        left, arguments, ..
    } = f.context().tree.get(node_id)
    else {
        return Err(FormatError::SyntaxError {
            message: "unexpected expression kind for call-chain routing",
        });
    };

    // long curried calls stay under the direct call owner
    if expression_is_long_curried_call(f.context(), node_id) {
        return format_call_expression(f, node_id);
    }

    let should_route_to_chain = call_should_route_to_chain(f.context(), node_id, *left, arguments);
    if should_route_to_chain {
        format_expression_chain(f, node_id)?;
    } else {
        format_call_expression(f, node_id)?;
    }

    Ok(())
}

/// Format a must expression without chain routing.
fn format_must_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    position: PostfixPosition,
    left: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_postfix_base_expression(f, left)?;
    if position == PostfixPosition::Indirect {
        write!(f, [token(".")])?;
    }
    write!(f, [token("!")])?;

    Ok(())
}

/// Return whether one postfix expression should route to the chain owner.
fn postfix_expression_should_route_to_chain(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
) -> bool {
    chain_has_call_like_expression(context, left_id)
}
