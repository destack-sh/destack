use crate::format::chain::{MemberChain, transparent_inner_expression};
use crate::format::operator::format_generic_argument_list;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, BinaryOperator, Expression, GenericArgument, LocalNodeId, ScalarLiteral,
};
use destack_fir::format::{Buffer, FormatNodes, Formatter as FirFormatter, VecBuffer};

/// Return whether one argument expression is short enough to keep a call attached.
fn is_short_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    threshold: u32,
) -> bool {
    let argument_expression_id = match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => return false,
    };

    is_short_expression(context, argument_expression_id, threshold)
}

/// Return whether one expression is short enough to keep a call attached.
fn is_short_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    threshold: u32,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Identifier { name } => context.strings.get(*name).len() <= threshold as usize,
        Expression::Unary { right, .. } => is_short_expression(context, *right, threshold),
        Expression::ScalarLiteral(
            ScalarLiteral::Boolean(_)
            | ScalarLiteral::Integer(_)
            | ScalarLiteral::Bigint(_)
            | ScalarLiteral::Float(_),
        )
        | Expression::This => true,
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            context.strings.get(*string_id).len() <= threshold as usize
        }
        Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) => {
            context.strings.get(*content).len() <= threshold as usize
        }
        Expression::TemplateExpression { .. } => !context.node_has_newline(expression_id),
        Expression::Call {
            left, arguments, ..
        } => {
            arguments.is_empty()
                && matches!(
                    context.tree.get(transparent_inner_expression(context, *left)),
                    Expression::Identifier { name }
                        if context.strings.get(*name).len()
                            <= threshold.saturating_sub(2) as usize
                )
        }
        _ => false,
    }
}

/// Return whether one generic argument list is complex enough to break a call chain.
fn is_complex_generic_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> bool {
    if generic_arguments.len() > 1 {
        return true;
    }

    let Some(argument_id) = generic_arguments.first().copied() else {
        return false;
    };
    let argument_expression_id = match f.context().tree.get(argument_id) {
        GenericArgument::Type { .. } => return true,
        GenericArgument::Value { value } => *value,
        GenericArgument::Error => return false,
    };
    let argument_expression_id = transparent_inner_expression(f.context(), argument_expression_id);

    if matches!(
        f.context().tree.get(argument_expression_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
            ..
        } | Expression::Type { .. }
    ) {
        return true;
    }

    let mut buffer = VecBuffer::new(f.state_mut());
    let formatter = &mut FirFormatter::new(&mut buffer);
    if format_generic_argument_list(formatter, generic_arguments).is_err() {
        return true;
    }

    buffer.into_vec().as_slice().will_break()
}

/// Return whether one call or member chain is awkward to break inside an assignment shell.
pub(crate) fn is_poorly_breakable_member_or_call_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let threshold = u32::from(f.context().options.line_width) / 4;
    let root_expression_id = transparent_inner_expression(f.context(), expression_id);
    let mut current_expression_id = root_expression_id;
    let mut is_chain = false;
    let mut has_simple_head = false;
    let mut call_expression_ids = Vec::new();
    let mut call_generic_argument_groups = Vec::<Vec<LocalNodeId<GenericArgument>>>::new();

    loop {
        current_expression_id = match f.context().tree.get(current_expression_id) {
            // call
            Expression::Call {
                left,
                generic_arguments,
                ..
            } => {
                is_chain = true;
                call_expression_ids.push(current_expression_id);
                call_generic_argument_groups.push(generic_arguments.clone());
                transparent_inner_expression(f.context(), *left)
            }

            // instantiation
            Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                is_chain = true;
                if is_complex_generic_arguments(f, generic_arguments) {
                    return false;
                }

                transparent_inner_expression(f.context(), *left)
            }

            // member
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => {
                is_chain = true;
                transparent_inner_expression(f.context(), *left)
            }

            // simple heads
            Expression::Identifier { .. } | Expression::This => {
                has_simple_head = true;
                break;
            }

            // non-chains
            _ => break,
        };
    }

    // non-simple chain heads do not use this shell shortcut
    if !is_chain || !has_simple_head {
        return false;
    }

    // any comments inside the root chain already force the safer layout
    if f.context()
        .comments()
        .has_comment_in_span(f.context().span(root_expression_id))
    {
        return false;
    }

    // pure member chains are cheap to keep attached
    if call_expression_ids.is_empty() {
        return true;
    }

    // comments on the outer call break the shortcut
    if f.context()
        .comments()
        .has_comment_in_span(f.context().span(call_expression_ids[0]))
    {
        return false;
    }

    // breakable calls defeat the shortcut
    for (index, call_expression_id) in call_expression_ids.iter().copied().enumerate() {
        let Expression::Call { arguments, .. } = f.context().tree.get(call_expression_id) else {
            continue;
        };

        let is_breakable_call = match arguments.len() {
            0 => false,
            1 => {
                let argument_id = arguments[0];
                !is_short_argument(f.context(), argument_id, threshold)
            }
            _ => true,
        };
        if is_breakable_call {
            return false;
        }

        if let Some(generic_arguments) = call_generic_argument_groups.get(index)
            && is_complex_generic_arguments(f, generic_arguments)
        {
            return false;
        }
    }

    // multi-group chains already have enough internal structure
    MemberChain::tail_group_count(f.context(), root_expression_id)
        .map(|count| count <= 1)
        .unwrap_or(true)
}
