use super::super::*;
use destack_fir::write;

/// Format a `new` expression.
pub(super) fn format_new_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let tree = f.context().tree;

    // unwrap redundant parenthesized member callees
    let mut left = left;
    if let Expression::Parenthesized { expression } = tree.get(left)
        && parenthesized_should_unwrap(
            f.context(),
            left,
            *expression,
            ParenthesizedUnwrapPolicy::NewMemberCallee,
        )
    {
        left = *expression;
    }

    // preserve parenthesized index chains like `(foo[bar])`
    if let Some((base_expression, indices)) =
        extract_parenthesized_index_chain(f.context().tree, left)
    {
        let formatted_left = format_with(move |f| {
            write!(f, [token("("), base_expression])?;
            for index in &indices {
                write!(f, [token("["), *index, token("]")])?;
            }
            write!(f, [token(")")])
        });

        write!(f, [token("new"), space(), formatted_left])?;
    }
    // otherwise use regular member-callee wrapping rules
    else {
        let should_wrap_member_callee = matches!(
            tree.get(left),
            Expression::Member {
                left: member_left,
                ..
            } | Expression::PrivateMember {
                left: member_left,
                ..
            } if parenthesized_prefers_new_member_callee_parentheses(
                f.context(),
                *member_left
            )
        );

        // keep wrapped member callee when required
        if should_wrap_member_callee {
            write!(f, [token("new"), space(), token("("), left, token(")")])?;
        }
        // otherwise write regular new callee
        else {
            write!(f, [token("new"), space(), left])?;
        }
    }

    // write static type arguments
    if let Some(static_arguments) = static_arguments {
        format_static_argument_list(f, static_arguments)?;
    }

    // write dynamic argument list
    format_call_arguments(f, node_id, dynamic_arguments)?;

    Ok(())
}
