use crate::analysis::write_plain_call_argument_or_node;
use crate::expression::{
    Argument, Declaration, DestackFormatContext, DestackFormatter, Expression, FormatResult,
    FunctionKind, LocalNodeId, argument_value_id, hard_line_break, token,
    transparent_inner_expression,
};
use destack_fir::format::Buffer;
use destack_fir::write;

/// Write one single callback argument with a trailing separator wrap.
pub(super) fn write_single_callback_argument_wrapped_with_trailing_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    write_plain_call_argument_or_node(f, argument_id)?;
    write!(f, [token(","), hard_line_break(), token(")")])
}

/// Return whether single callback rendering should keep hugging with a trailing separator wrap.
fn callback_argument_has_call_chain_body(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };
    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = context.tree.get(*declaration_id)
    else {
        return false;
    };
    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    let mut expression_id = transparent_inner_expression(context, *body_id);
    let mut saw_call_like = false;
    let mut saw_chain_member = false;
    loop {
        match context.tree.get(expression_id) {
            Expression::Call { left, .. } | Expression::Instantiation { left, .. } => {
                saw_call_like = true;
                expression_id = transparent_inner_expression(context, *left);
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => {
                saw_chain_member = true;
                expression_id = transparent_inner_expression(context, *left);
            }
            _ => break,
        }
    }

    saw_call_like && saw_chain_member
}

/// Return whether single callback rendering should keep hugging with a trailing separator wrap.
pub(super) fn single_callback_argument_prefers_trailing_separator_wrap(
    context: &DestackFormatContext<'_>,
    _call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    callback_argument_has_call_chain_body(context, argument_id)
}
