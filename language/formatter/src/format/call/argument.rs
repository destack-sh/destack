use crate::format::annotation::{infix_or_postfix_annotations, prefix_annotations};
use crate::format::declaration::{
    FormatFunctionDeclarationOptions, GroupedCallArgumentLayout,
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
    format_function_declaration_with_options,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Argument, Declaration, Expression, FunctionKind, LocalNodeId,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{hard_line_break, space, token};
use destack_fir::write;

/// Return whether an argument can be emitted directly without argument-node formatting.
pub(crate) fn argument_is_plain_call_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    !context.has_annotation(argument_id)
        && matches!(
            context.tree.get(argument_id),
            Argument::Named {
                modifiers: None,
                ..
            } | Argument::Labeled {
                modifiers: None,
                ..
            } | Argument::Positional {
                modifiers: None,
                ..
            } | Argument::Spread {
                modifiers: None,
                ..
            }
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
                Annotation::Decorator { position, .. } => matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
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

    let has_lambda_value = argument_contains_lambda_value(f.context(), argument);

    if has_lambda_value || argument_has_prefix_annotation(f.context(), node_id) {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    write_argument_with_modifiers_and_value(argument, false, None, f)?;

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

    if argument_is_plain_call_argument(f.context(), argument_id)
        && !argument_contains_lambda_value(f.context(), argument)
    {
        return write_plain_call_argument(f, argument_id);
    }

    let has_lambda_value = argument_contains_lambda_value(f.context(), argument);

    if has_lambda_value || argument_has_prefix_annotation(f.context(), argument_id) {
        write!(f, [prefix_annotations(f.context(), argument_id)])?;
    }

    write_argument_with_modifiers_and_value(
        argument,
        false,
        Some(grouped_call_argument_layout),
        f,
    )?;

    write!(f, [infix_or_postfix_annotations(f.context(), argument_id)])?;

    Ok(())
}

/// Return whether this argument wraps a lambda declaration expression.
fn argument_contains_lambda_value(context: &DestackFormatContext<'_>, argument: &Argument) -> bool {
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => return false,
    };

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

/// Write one argument expression value.
fn write_argument_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    grouped_call_argument_layout: Option<GroupedCallArgumentLayout>,
) -> FormatResult<()> {
    let Expression::Declaration(declaration_id) = f.context().tree.get(value_id) else {
        return write!(f, [value_id]);
    };
    let Declaration::Function {
        descriptor,
        signature,
        body,
    } = f.context().tree.get(*declaration_id)
    else {
        return write!(f, [value_id]);
    };

    if signature.kind != FunctionKind::Lambda {
        return write!(f, [value_id]);
    }

    format_function_declaration_with_options(
        f,
        *declaration_id,
        descriptor,
        signature,
        body,
        FormatFunctionDeclarationOptions {
            grouped_call_argument_layout,
        },
    )
}

/// Write one argument with modifiers and value payload.
fn write_argument_with_modifiers_and_value<'ast>(
    argument: &Argument,
    force_break_after_lambda_prefix_comment: bool,
    grouped_call_argument_layout: Option<GroupedCallArgumentLayout>,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    match argument {
        Argument::Named {
            modifiers,
            name,
            value,
        } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;
            write!(f, [name])?;
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            write!(f, [token(":"), space()])?;
            write_argument_value(f, *value, grouped_call_argument_layout)?;
        }
        Argument::Labeled {
            modifiers,
            label,
            value,
        } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;
            write!(f, [label])?;
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            write!(f, [token(":"), space()])?;
            write_argument_value(f, *value, grouped_call_argument_layout)?;
        }
        Argument::Positional { modifiers, value } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;

            if force_break_after_lambda_prefix_comment {
                write!(f, [hard_line_break()])?;
            }

            write_argument_value(f, *value, grouped_call_argument_layout)?;
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;
        }
        Argument::Spread {
            modifiers,
            label,
            value,
        } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;
            write!(f, [token("...")])?;

            if let Some(label) = label {
                write!(f, [label])?;
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                write!(f, [token(":"), space()])?;
                write_argument_value(f, *value, grouped_call_argument_layout)?;
            } else {
                if force_break_after_lambda_prefix_comment {
                    write!(f, [hard_line_break()])?;
                }

                write_argument_value(f, *value, grouped_call_argument_layout)?;
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            }
        }
        Argument::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    Ok(())
}
