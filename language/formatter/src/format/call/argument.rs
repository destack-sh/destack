use crate::format::annotation::{infix_or_postfix_annotations, prefix_annotations};
use crate::format::declaration::GroupedCallArgumentLayout;
use crate::{Decorator, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{Argument, DecoratorPosition, Expression, LocalNodeId};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{space, token};
use destack_fir::write;

/// Return whether an argument can be emitted directly without argument-node formatting.
pub(crate) fn argument_is_plain_call_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    !context.has_annotation(argument_id)
        && matches!(
            context.tree.get(argument_id),
            Argument::Named { .. }
                | Argument::Labeled { .. }
                | Argument::Positional { .. }
                | Argument::Spread { .. }
        )
}

/// Write one call argument that is known to be plain.
pub(crate) fn write_plain_call_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(f, [prefix_annotations(f.context(), argument_id)])?;

    match f.context().tree.get(argument_id) {
        Argument::Named { name, value, .. } => {
            write!(f, [*name, token(":"), space(), *value])?;
        }
        Argument::Labeled { label, value, .. } => {
            write!(f, [*label, token(":"), space(), *value])?;
        }
        Argument::Positional { value, .. } => {
            write!(f, [*value])?;
        }
        Argument::Spread {
            label: Some(label),
            value,
            ..
        } => {
            write!(f, [token("..."), *label, token(":"), space(), *value])?;
        }
        Argument::Spread {
            label: None, value, ..
        } => {
            write!(f, [token("..."), *value])?;
        }
        Argument::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    write!(f, [infix_or_postfix_annotations(f.context(), argument_id)])?;

    Ok(())
}

/// Return whether an argument has a prefix annotation.
fn argument_has_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    !context.raw_prefix_doc_comments_for(argument_id).is_empty()
        || context
            .annotation_ids(argument_id)
            .iter()
            .copied()
            .any(|annotation_id| match context.annotation(annotation_id) {
                Decorator { position, .. } => matches!(
                    position,
                    DecoratorPosition::LinePrefix | DecoratorPosition::BlockPrefix
                ),
            })
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        node_id: LocalNodeId<Argument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write_call_argument_node_body(f, node_id)
    }
}

/// Write one argument node without list-level trailing comment ownership.
pub(crate) fn write_call_argument_node_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    let argument = f.context().tree.get(node_id);

    if argument_is_plain_call_argument(f.context(), node_id) {
        write_plain_call_argument(f, node_id)?;
        return Ok(());
    }

    if argument_has_prefix_annotation(f.context(), node_id) {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    write_argument_with_value(argument, None, f)?;

    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

    Ok(())
}

/// Write one grouped call argument.
pub(crate) fn write_grouped_call_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    grouped_call_argument_layout: GroupedCallArgumentLayout,
) -> FormatResult<()> {
    let argument = f.context().tree.get(argument_id);

    if argument_is_plain_call_argument(f.context(), argument_id) {
        return write_plain_call_argument(f, argument_id);
    }

    if argument_has_prefix_annotation(f.context(), argument_id) {
        write!(f, [prefix_annotations(f.context(), argument_id)])?;
    }

    write_argument_with_value(argument, Some(grouped_call_argument_layout), f)?;

    write!(f, [infix_or_postfix_annotations(f.context(), argument_id)])?;

    Ok(())
}

/// Write one argument expression value.
fn write_argument_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    _grouped_call_argument_layout: Option<GroupedCallArgumentLayout>,
) -> FormatResult<()> {
    write!(f, [value_id])
}

/// Write one argument with its value payload.
fn write_argument_with_value<'ast>(
    argument: &Argument,
    grouped_call_argument_layout: Option<GroupedCallArgumentLayout>,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    match argument {
        Argument::Named { name, value } => {
            write!(f, [name])?;
            write!(f, [token(":"), space()])?;
            write_argument_value(f, *value, grouped_call_argument_layout)?;
        }
        Argument::Labeled { label, value } => {
            write!(f, [label])?;
            write!(f, [token(":"), space()])?;
            write_argument_value(f, *value, grouped_call_argument_layout)?;
        }
        Argument::Positional { value } => {
            write_argument_value(f, *value, grouped_call_argument_layout)?;
        }
        Argument::Spread { label, value } => {
            write!(f, [token("...")])?;

            if let Some(label) = label {
                write!(f, [label])?;
                write!(f, [token(":"), space()])?;
                write_argument_value(f, *value, grouped_call_argument_layout)?;
            } else {
                write_argument_value(f, *value, grouped_call_argument_layout)?;
            }
        }
        Argument::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    Ok(())
}
