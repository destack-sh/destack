use super::super::super::timing::tags;
use super::super::*;
use super::assign::format_assign_expression;
use super::binary::{format_binary_expression, format_type_binary_expression};
use super::r#new::format_new_expression;
use destack_fir::write;

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
                let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);
                if needs_space {
                    if right_needs_await_or_yield_grouping {
                        write!(f, [operator, space(), token("("), right, token(")")])?;
                    } else {
                        write!(f, [operator, space(), right])?;
                    }
                } else if right_needs_await_or_yield_grouping {
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
                write!(f, [right, token(" as const")])?;
            }
            TypeUnaryOperator::AsComptime => {
                write!(f, [right, token(" as comptime")])?;
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
        Expression::Index { left, .. } => {
            format_index_or_chain_expression(f, node_id, *left)?;
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
    left: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let left_is_instantiation =
        matches!(f.context().tree.get(left), Expression::Instantiation { .. });
    if is_expression_chain(f.context().tree, node_id) && !left_is_instantiation {
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
    if is_expression_chain(f.context().tree, node_id)
        || call_prefers_chain_format(f.context(), node_id)
    {
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
