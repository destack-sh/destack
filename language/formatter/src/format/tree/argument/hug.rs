use crate::expression::{
    Argument, Declaration, DestackFormatContext, DestackFormatter, Expression, FormatResult,
    FunctionKind, HugOptions, LocalNodeId, NodeTree, argument_value, block_indent, format_with,
    group, hard_line_break, if_group_breaks, lambda_expression_should_break,
    soft_line_break_or_space, space, token, transparent_inner_expression,
};
use destack_fir::format::{Buffer, Format, GroupId};
use destack_fir::prelude::expand_parent;
use destack_fir::{format_args, write};

pub(crate) fn is_huggable_expression(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    config: &HugOptions,
) -> bool {
    match tree.get(expression_id) {
        Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. } => true,
        Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
            // arrow functions stay hugged when used as the only call argument
            if let Declaration::Function {
                signature,
                body: Some(_),
                ..
            } = tree.get(*declaration_id)
            {
                signature.kind == FunctionKind::Lambda
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Store arrow-specific layout facts for hugged argument formatting.
#[derive(Clone, Copy, Default)]
struct HuggedArrowLayoutSignals {
    is_arrow_function: bool,
    arrow_force_expand: bool,
    arrow_trailing_line_break_if_breaks: bool,
    arrow_trailing_comma_if_breaks: bool,
}

/// Collect arrow-specific layout signals for one hugged argument value.
fn collect_hugged_arrow_layout_signals(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
    config: &HugOptions,
) -> HuggedArrowLayoutSignals {
    if !config.allow_arrow_functions {
        return HuggedArrowLayoutSignals::default();
    }

    let tree = context.tree;
    let arrow_declaration_id = match tree.get(value_id) {
        Expression::Declaration(declaration_id) => match tree.get(*declaration_id) {
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda => {
                Some(*declaration_id)
            }
            _ => None,
        },
        _ => None,
    };

    let Some(declaration_id) = arrow_declaration_id else {
        return HuggedArrowLayoutSignals::default();
    };

    let (arrow_body_is_block, arrow_body_is_tree, arrow_body_is_lambda) =
        if let Declaration::Function {
            body: Some(body_id),
            ..
        } = tree.get(declaration_id)
        {
            let body_id = transparent_inner_expression(context, *body_id);
            let body_expr = tree.get(body_id);
            let is_block = matches!(body_expr, Expression::Block(_));
            let is_tree = matches!(body_expr, Expression::TreeExpression { .. });
            let is_lambda = matches!(
                body_expr,
                Expression::Declaration(nested_declaration_id)
                    if matches!(
                        tree.get(*nested_declaration_id),
                        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
                    )
            );
            (is_block, is_tree, is_lambda)
        } else {
            (false, false, false)
        };

    let arrow_force_expand = lambda_expression_should_break(context, declaration_id);
    let arrow_trailing_line_break_if_breaks =
        !arrow_body_is_block && !arrow_body_is_tree && !arrow_body_is_lambda;
    let arrow_trailing_comma_if_breaks =
        arrow_trailing_line_break_if_breaks && !arrow_body_is_lambda;

    HuggedArrowLayoutSignals {
        is_arrow_function: true,
        arrow_force_expand,
        arrow_trailing_line_break_if_breaks,
        arrow_trailing_comma_if_breaks,
    }
}

/// Choose between inline and hugged candidate docs for one single hugged argument.
#[allow(clippy::too_many_arguments)]
fn choose_hugged_argument_layout<'ast, InlineDoc, HuggedDoc>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    value_id: LocalNodeId<Expression>,
    force_expand: bool,
    signals: HuggedArrowLayoutSignals,
    inline_format: InlineDoc,
    hugged_format: HuggedDoc,
) -> FormatResult<()>
where
    InlineDoc: Format<DestackFormatContext<'ast>>,
    HuggedDoc: Format<DestackFormatContext<'ast>>,
{
    // forced expansion always selects hugged output
    if force_expand {
        hugged_format.format(f)?;
        return Ok(());
    }

    // context-based default selection
    let is_annotated =
        f.context().has_annotation(argument_id) || f.context().has_annotation(value_id);
    let should_hug = is_annotated || signals.arrow_force_expand;
    if should_hug {
        f.context()
            .increment_counter("profile.jsx.hug.by_context.hug", 1);
        hugged_format.format(f)?;
    } else {
        f.context()
            .increment_counter("profile.jsx.hug.by_context.inline", 1);
        inline_format.format(f)?;
    }

    Ok(())
}

/// Format a single-element argument list with hugging for expandable elements.
///
/// When a single object/array (or arrow function for calls) is the only argument,
/// format as `foo({...})` instead of `foo(\n    {...},\n)`.
/// Returns true if hugging was applied, false if regular list_like should be used.
pub(crate) fn format_hugged<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<Argument>],
    config: HugOptions,
    group_id: Option<GroupId>,
    force_expand: bool,
) -> FormatResult<bool> {
    // only hug single positional arguments
    if arguments.len() != 1 {
        return Ok(false);
    }

    // resolve and normalize the single argument value
    let argument_id = arguments[0];
    let Some(value_id) = argument_value(f.context().tree, argument_id) else {
        return Ok(false);
    };
    let value_id = transparent_inner_expression(f.context(), value_id);

    // guard non-huggable value kinds
    if !is_huggable_expression(f.context().tree, value_id, &config) {
        return Ok(false);
    }

    // multiline empty collections are not stable hugging candidates
    if !force_expand
        && f.context().node_has_newline(value_id)
        && match f.context().tree.get(value_id) {
            Expression::ObjectExpression { properties, .. } => properties.is_empty(),
            Expression::ArrayExpression { elements } => elements.is_empty(),
            _ => false,
        }
    {
        return Ok(false);
    }

    // multiline collection values can opt out of hugging by configuration
    if !force_expand
        && !config.allow_multiline_collection
        && f.context().node_has_newline(value_id)
        && matches!(
            f.context().tree.get(value_id),
            Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
        )
    {
        return Ok(false);
    }

    // collect arrow-specific layout facts once
    let signals = collect_hugged_arrow_layout_signals(f.context(), value_id, &config);
    let trailing_if_breaks = config.trailing_if_breaks;

    // build inline candidate doc
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let inline_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [token(config.open), argument_id])?;
            if config.force_trailing {
                write!(f, [token(",")])?;
            }
            write!(f, [token(config.close)])
        });

        if let Some(group_id) = group_id {
            group(&inline_inner).with_id(Some(group_id)).format(f)
        } else {
            write!(f, [inline_inner])
        }
    });

    // build hugged candidate doc
    let hugged_format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // argument annotations
        if config.handle_annotations {
            if signals.is_arrow_function {
                write!(f, [f.context().block_prefix_annotations(argument_id)])?;
            } else {
                write!(f, [f.context().any_prefix_annotations(argument_id)])?;
            }
        }

        // force expansion when arrow policy requires it
        if signals.arrow_force_expand {
            write!(f, [expand_parent()])?;
        }

        // opening delimiter
        write!(f, [token(config.open)])?;

        // value payload
        match f.context().tree.get(value_id) {
            Expression::ObjectExpression { ty, properties } => {
                if let Some(ty) = ty {
                    write!(f, [ty, space()])?;
                }
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                token("{"),
                                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    f.join_with(&format_args![
                                        token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(properties)
                                    .finish()?;
                                    write!(f, [if_group_breaks(&token(","))])
                                })),
                                token("}")
                            ]
                        )
                    }))
                    .should_expand(true)]
                )?;
            }
            Expression::ArrayExpression { elements } => {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                token("["),
                                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    f.join_with(&format_args![
                                        token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(elements)
                                    .finish()?;
                                    write!(f, [if_group_breaks(&token(","))])
                                })),
                                token("]")
                            ]
                        )
                    }))
                    .should_expand(true)]
                )?;
            }
            Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
                if let Some(group_id) = group_id {
                    write!(
                        f,
                        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(f, [declaration_id])
                        }))
                        .with_id(Some(group_id))]
                    )?;
                } else {
                    write!(f, [declaration_id])?;
                }
            }
            _ => {
                write!(f, [value_id])?;
            }
        }

        // trailing comma behavior
        if config.force_trailing {
            write!(f, [token(",")])?;
        } else if signals.arrow_trailing_comma_if_breaks || trailing_if_breaks {
            let comma = token(",");
            let trailing_comma = if let Some(group_id) = group_id {
                if_group_breaks(&comma).with_group_id(Some(group_id))
            } else {
                if_group_breaks(&comma)
            };
            write!(f, [trailing_comma])?;
        }

        // trailing line break behavior for arrow values
        if signals.arrow_trailing_line_break_if_breaks {
            let line_break = hard_line_break();
            let break_doc = if let Some(group_id) = group_id {
                if_group_breaks(&line_break).with_group_id(Some(group_id))
            } else {
                if_group_breaks(&line_break)
            };
            write!(f, [break_doc])?;
        }

        // closing delimiter
        write!(f, [token(config.close)])?;

        // trailing annotations
        if config.handle_annotations {
            write!(
                f,
                [f.context().any_infix_or_postfix_annotations(argument_id)]
            )?;
        }
        Ok(())
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if trailing_if_breaks || group_id.is_some() {
            write!(
                f,
                [group(&hugged_format_inner)
                    .with_id(group_id)
                    .should_expand(signals.arrow_force_expand)]
            )?;
        } else {
            write!(f, [hugged_format_inner])?;
        }
        Ok(())
    });

    // choose final candidate
    choose_hugged_argument_layout(
        f,
        argument_id,
        value_id,
        force_expand,
        signals,
        inline_format,
        hugged_format,
    )?;

    Ok(true)
}
