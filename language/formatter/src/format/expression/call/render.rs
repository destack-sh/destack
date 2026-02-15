use super::super::*;
use super::profile::*;
use crate::timing::tags;
use destack_fir::write;

/// Format a call expression.
#[inline]
pub(crate) fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f.context().timing_scope(tags::FORMAT_EXPRESSION_CALL);

    if let Expression::Call {
        position,
        left,
        static_arguments,
        dynamic_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(static_arguments) = static_arguments {
            format_static_argument_list(f, static_arguments)?;
        }

        format_call_arguments(f, node_id, dynamic_arguments)?;
    } else {
        debug_assert!(false, "unexpected expression kind for call formatter");
    }
    Ok(())
}

/// Format an instantiation expression.
#[inline]
pub(crate) fn format_instantiation_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Instantiation {
        left,
        static_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        format_static_argument_list(f, static_arguments)?;
    } else {
        debug_assert!(
            false,
            "unexpected expression kind for instantiation formatter"
        );
    }
    Ok(())
}
