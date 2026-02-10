use super::super::*;
use destack_fir::write;

/// Format a `new` expression including deferred empty argument comments.
pub(super) fn format_new_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
    dynamic_arguments: &Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    let tree = f.context().tree;

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

    let has_deferred_empty_argument_comments = if dynamic_arguments.is_empty() {
        let (inline_argument_comment, line_argument_comment, trailing_optional_comment) =
            collect_deferred_empty_call_boundary_comments(f.context(), node_id);
        inline_argument_comment.is_some()
            || line_argument_comment.is_some()
            || trailing_optional_comment.is_some()
    } else {
        false
    };

    let left_without_infix_or_postfix_annotations = format_with(|f| {
        let directive = directive_for_node(f.context(), left);
        write!(f, [f.context().any_prefix_annotations(left)])?;
        format_expression(f, left, f.context().tree.get(left), directive)?;
        Ok(())
    });

    let mut formatted_left = None;
    if let Some((base_expression, indices)) =
        extract_parenthesized_index_chain(f.context().tree, left)
    {
        formatted_left = Some(format_with(move |f| {
            write!(f, [token("("), base_expression])?;
            for index in &indices {
                write!(f, [token("["), *index, token("]")])?;
            }
            write!(f, [token(")")])
        }));
    }

    if let Some(formatted_left) = formatted_left {
        write!(f, [token("new"), space(), formatted_left])?;
    } else {
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
        } else if has_deferred_empty_argument_comments {
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
    if let Some(static_arguments) = static_arguments {
        format_static_argument_list(f, static_arguments)?;
    }
    format_call_dynamic_arguments_with_deferred_comments(f, node_id, dynamic_arguments)?;

    Ok(())
}
