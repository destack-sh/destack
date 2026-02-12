use super::super::*;
use destack_fir::write;

/// Return whether an empty-argument new call has deferred boundary comments.
fn new_call_has_deferred_empty_argument_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if !dynamic_arguments.is_empty() {
        return false;
    }

    let (inline_argument_comment, line_argument_comment, trailing_optional_comment) =
        collect_deferred_empty_call_boundary_comments(context, node_id);
    inline_argument_comment.is_some()
        || line_argument_comment.is_some()
        || trailing_optional_comment.is_some()
}

/// Return a formatted left callee that emits only prefix annotations.
fn format_new_left_without_infix_or_postfix_annotations<'ast>(
    left: LocalNodeId<Expression>,
) -> impl Format<DestackFormatContext<'ast>> {
    format_with(move |f| {
        let directive = directive_for_node(f.context(), left);
        write!(f, [f.context().any_prefix_annotations(left)])?;
        format_expression(f, left, f.context().tree.get(left), directive)?;
        Ok(())
    })
}

/// Format a `new` expression including deferred empty argument comments.
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

    // collect deferred empty-argument comment state
    let has_deferred_empty_argument_comments =
        new_call_has_deferred_empty_argument_comments(f.context(), node_id, dynamic_arguments);

    // prepare an annotation-safe left formatter for deferred comment cases
    let left_without_infix_or_postfix_annotations =
        format_new_left_without_infix_or_postfix_annotations(left);

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
            if has_deferred_empty_argument_comments {
                write!(
                    f,
                    [
                        token("new"),
                        space(),
                        token("("),
                        left_without_infix_or_postfix_annotations,
                        token(")")
                    ]
                )?;
            } else {
                write!(f, [token("new"), space(), token("("), left, token(")")])?;
            }
        }
        // otherwise write regular new callee
        else if has_deferred_empty_argument_comments {
            write!(
                f,
                [
                    token("new"),
                    space(),
                    left_without_infix_or_postfix_annotations
                ]
            )?;
        } else {
            write!(f, [token("new"), space(), left])?;
        }
    }

    // write static type arguments
    if let Some(static_arguments) = static_arguments {
        format_static_argument_list(f, static_arguments)?;
    }

    // write dynamic argument list with deferred boundary comments
    format_call_dynamic_arguments_with_deferred_comments(f, node_id, dynamic_arguments)?;

    Ok(())
}
