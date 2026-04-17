use super::list::{call_argument_lines_before, write_call_argument_in_list};
use super::pattern::argument_expression_id;
use crate::format::chain::{SimpleArgument, transparent_inner_expression};
use crate::format::context::DestackFormatterCommentExt;
use crate::format::declaration::GroupedCallArgumentLayout;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Declaration, Expression, FunctionKind, GenericArgument, LocalNodeId, ScalarLiteral,
    TypeExpression, UnaryOperator,
};
use destack_fir::format::{
    BestFittingMode, Buffer, FormatNode as FirNode, FormatNodes, FormatResult, GroupId,
};
use destack_fir::prelude::{
    empty_line, format_with, group, soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{best_fitting, format_args, write};

/// Return whether any argument carries annotations.
fn arguments_have_annotations(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    arguments
        .iter()
        .copied()
        .any(|argument_id| context.has_annotation(argument_id))
}

/// Return whether one expression is a function declaration expression.
fn expression_is_function_argument(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = ctx.tree.get(expression_id) else {
        return false;
    };

    matches!(ctx.tree.get(*declaration_id), Declaration::Function { .. })
}

/// Return whether one lambda or function can group as the first call argument.
fn expression_is_groupable_first_argument(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = ctx.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Function(function) = ctx.tree.get(*declaration_id) else {
        return false;
    };
    let Some(body_id) = function.body else {
        return false;
    };

    if function.signature.kind != FunctionKind::Lambda {
        return true;
    }

    let body_id = transparent_inner_expression(ctx, body_id);
    matches!(ctx.tree.get(body_id), Expression::Block(_))
}

/// Return whether one expression body can group in call-argument layout.
fn expression_can_group_lambda_body(
    ctx: &DestackFormatContext<'_>,
    body_id: LocalNodeId<Expression>,
    is_lambda_recursion: bool,
) -> bool {
    let body_id = transparent_inner_expression(ctx, body_id);

    match ctx.tree.get(body_id) {
        Expression::Block(_)
        | Expression::ObjectExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::TreeExpression { .. } => true,
        Expression::Declaration(_) => {
            !is_lambda_recursion && expression_can_group_function_argument(ctx, body_id, true)
        }
        Expression::Call { .. } | Expression::If { .. } => !is_lambda_recursion,
        _ => false,
    }
}

/// Return whether one expression can participate in grouped call-argument layout.
fn expression_can_group_function_argument(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    is_lambda_recursion: bool,
) -> bool {
    let Expression::Declaration(declaration_id) = ctx.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Function(function) = ctx.tree.get(*declaration_id) else {
        return false;
    };
    let Some(body_id) = function.body else {
        return false;
    };

    if function.signature.kind != FunctionKind::Lambda {
        return true;
    }

    expression_can_group_lambda_body(ctx, body_id, is_lambda_recursion)
}

/// Return whether two expressions share the same grouped-argument kind.
fn expressions_share_grouped_layout_kind(
    ctx: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    match (ctx.tree.get(left_id), ctx.tree.get(right_id)) {
        (Expression::ObjectExpression { .. }, Expression::ObjectExpression { .. })
        | (Expression::ArrayExpression { .. }, Expression::ArrayExpression { .. })
        | (Expression::As { .. }, Expression::As { .. })
        | (Expression::Satisfies { .. }, Expression::Satisfies { .. }) => true,
        _ => {
            expression_is_function_argument(ctx, left_id)
                && expression_is_function_argument(ctx, right_id)
        }
    }
}

/// Return whether one type expression is simple enough for grouped call layout.
fn type_expression_is_simple(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    let expression_id = array_type_element_expression(ctx, expression_id);
    let expression_id = single_generic_argument_type_expression(ctx, expression_id);

    match ctx.tree.get(expression_id) {
        TypeExpression::ScalarLiteral { .. }
        | TypeExpression::Literal { .. }
        | TypeExpression::Intrinsic
        | TypeExpression::Const
        | TypeExpression::This => true,
        TypeExpression::Reference {
            generic_arguments, ..
        }
        | TypeExpression::Member {
            generic_arguments, ..
        }
        | TypeExpression::Import {
            generic_arguments, ..
        } => generic_arguments.is_empty(),
        _ => false,
    }
}

/// Return one type expression after stripping up to two array suffixes.
fn array_type_element_expression(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> LocalNodeId<TypeExpression> {
    let mut expression_id = expression_id;

    // strip one or two array wrappers
    for _ in 0..2 {
        let TypeExpression::Array { element } = ctx.tree.get(expression_id) else {
            break;
        };

        expression_id = *element;
    }

    expression_id
}

/// Return one type expression after extracting one single generic argument.
fn single_generic_argument_type_expression(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> LocalNodeId<TypeExpression> {
    let generic_arguments = match ctx.tree.get(expression_id) {
        TypeExpression::Reference {
            generic_arguments, ..
        }
        | TypeExpression::Member {
            generic_arguments, ..
        }
        | TypeExpression::Import {
            generic_arguments, ..
        } => generic_arguments,
        _ => return expression_id,
    };

    // keep multi-argument generics intact
    if generic_arguments.len() != 1 {
        return expression_id;
    }

    let GenericArgument::Type { value } = ctx.tree.get(generic_arguments[0]) else {
        return expression_id;
    };

    *value
}

/// Return whether one expression can participate in grouped call-argument layout.
fn can_group_expression_argument(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(ctx, expression_id);

    match ctx.tree.get(expression_id) {
        Expression::ObjectExpression { .. } => {
            let Expression::ObjectExpression { properties, .. } = ctx.tree.get(expression_id)
            else {
                return false;
            };

            !properties.is_empty()
                || !ctx
                    .comments_in_range(ctx.span(expression_id).start, ctx.span(expression_id).end)
                    .is_empty()
        }
        Expression::ArrayExpression { .. } => {
            let Expression::ArrayExpression { elements, .. } = ctx.tree.get(expression_id) else {
                return false;
            };

            !elements.is_empty()
                || !ctx
                    .comments_in_range(ctx.span(expression_id).start, ctx.span(expression_id).end)
                    .is_empty()
        }
        Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
            can_group_expression_argument(ctx, transparent_inner_expression(ctx, *expression))
        }
        Expression::Declaration(_) => {
            expression_can_group_function_argument(ctx, expression_id, false)
        }
        _ => false,
    }
}

/// Return whether one expression is short enough to stay next to a grouped function argument.
fn is_relatively_short_argument(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if ctx.node_has_newline(expression_id) || ctx.has_annotation(expression_id) {
        return false;
    }

    let expression_id = transparent_inner_expression(ctx, expression_id);

    match ctx.tree.get(expression_id) {
        Expression::Binary { left, right, .. } => {
            let left_id = transparent_inner_expression(ctx, *left);
            let right_id = transparent_inner_expression(ctx, *right);

            SimpleArgument::from(left_id).is_simple_with_depth(ctx, 1)
                && SimpleArgument::from(right_id).is_simple_with_depth(ctx, 1)
        }
        Expression::As {
            expression,
            target_type,
        }
        | Expression::Satisfies {
            expression,
            target_type,
        } => {
            let left_id = transparent_inner_expression(ctx, *expression);
            let right_id = *target_type;

            type_expression_is_simple(ctx, right_id)
                && SimpleArgument::from(left_id).is_simple_with_depth(ctx, 1)
        }
        Expression::Call { arguments, .. } => match arguments.len() {
            0 => true,
            1 => SimpleArgument::from(expression_id).is_simple(ctx),
            _ => false,
        },
        Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }) => true,
        _ => SimpleArgument::from(expression_id).is_simple(ctx),
    }
}

/// Return whether the first argument should use grouped layout.
fn should_group_first_argument(
    ctx: &DestackFormatContext<'_>,
    first_id: LocalNodeId<Expression>,
    second_id: LocalNodeId<Expression>,
) -> bool {
    if !expression_is_groupable_first_argument(ctx, first_id) {
        return false;
    }

    if expression_is_function_argument(ctx, second_id)
        || matches!(ctx.tree.get(second_id), Expression::If { .. })
    {
        return false;
    }

    let first_span = ctx.span(first_id);
    let comments = ctx.comments();
    if comments.has_comment_before(first_span.start) {
        return false;
    }

    if !ctx
        .source_text()
        .next_non_whitespace_byte_is(first_span.end, b',')
    {
        return false;
    }

    if comments
        .comments_in_range(first_span.end, ctx.span(second_id).start)
        .iter()
        .any(|comment| comment.followed_by_newline())
    {
        return false;
    }

    is_relatively_short_argument(ctx, second_id)
}

/// Return whether the last argument should use grouped layout.
fn should_group_last_argument_impl(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    args_len: usize,
    penultimate_id: Option<LocalNodeId<Expression>>,
    last_id: LocalNodeId<Expression>,
) -> bool {
    if penultimate_id.is_some_and(|penultimate_id| {
        expressions_share_grouped_layout_kind(ctx, penultimate_id, last_id)
    }) {
        return false;
    }

    let last_span = ctx.span(last_id);
    let comments = ctx.comments();
    let has_comment_before_last = if let Some(penultimate_id) = penultimate_id {
        comments
            .comments_in_range(ctx.span(penultimate_id).end, last_span.start)
            .last()
            .is_some_and(|comment| {
                !comment.followed_by_newline()
                    && !ctx
                        .source_text()
                        .next_non_whitespace_byte_is(comment.span.end, b',')
            })
    } else {
        let call_span = ctx.span(call_node_id);
        comments.has_comment_in_range(call_span.start, last_span.start)
    };
    if has_comment_before_last {
        return false;
    }

    if comments
        .comments_after(last_span.end)
        .first()
        .is_some_and(|comment| {
            !ctx.source_text()
                .bytes_contain(last_span.end, comment.span.start, b')')
        })
    {
        return false;
    }

    match ctx.tree.get(last_id) {
        Expression::ArrayExpression { .. } if penultimate_id.is_some() => {
            if args_len == 2
                && penultimate_id.is_some_and(|penultimate_id| {
                    expression_is_zero_parameter_block_lambda(ctx, penultimate_id)
                })
            {
                return false;
            }

            !array_expression_is_concise(ctx, last_id)
        }
        _ => true,
    }
}

/// Return whether one expression is a zero-parameter lambda with a block body.
fn expression_is_zero_parameter_block_lambda(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = ctx.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Function(function) = ctx.tree.get(*declaration_id) else {
        return false;
    };
    let Some(body_id) = function.body else {
        return false;
    };

    let body_id = transparent_inner_expression(ctx, body_id);
    function.signature.kind == FunctionKind::Lambda
        && function.signature.parameters.is_empty()
        && matches!(ctx.tree.get(body_id), Expression::Block(_))
}

/// Return whether one array expression is concise enough to avoid grouped-last layout.
fn array_expression_is_concise(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(ctx, expression_id);
    let Expression::ArrayExpression { elements } = ctx.tree.get(expression_id) else {
        return false;
    };
    if elements.is_empty() {
        return false;
    }

    if !elements
        .iter()
        .copied()
        .all(|argument_id| argument_is_concise_numeric_array_element(ctx, argument_id))
    {
        return false;
    }

    !ctx.comments()
        .comments_before_iter(ctx.span(expression_id).end)
        .any(|comment| comment.is_line() && !comment.preceded_by_newline())
}

/// Return whether one array element is a concise numeric literal element.
fn argument_is_concise_numeric_array_element(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(expression_id) = argument_expression_id(ctx, argument_id) else {
        return false;
    };

    expression_is_concise_numeric_array_element(ctx, expression_id)
}

/// Return whether one expression is a concise numeric array element.
fn expression_is_concise_numeric_array_element(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(ctx, expression_id);

    match ctx.tree.get(expression_id) {
        Expression::ScalarLiteral(ScalarLiteral::Integer(_) | ScalarLiteral::Float(_)) => true,
        Expression::Unary { operator, right } => {
            matches!(
                operator,
                UnaryOperator::Plus
                    | UnaryOperator::Negate
                    | UnaryOperator::Not
                    | UnaryOperator::ElementwiseNot
            ) && matches!(
                ctx.tree.get(transparent_inner_expression(ctx, *right)),
                Expression::ScalarLiteral(ScalarLiteral::Integer(_) | ScalarLiteral::Float(_))
            ) && !ctx.comments().has_comment_in_span(ctx.span(expression_id))
        }
        _ => false,
    }
}

/// Return whether the last argument should use grouped layout.
fn should_group_last_argument(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    let Some(last_argument_id) = arguments.last().copied() else {
        return false;
    };
    let Some(last_id) = argument_expression_id(ctx, last_argument_id) else {
        return false;
    };

    let penultimate_id = arguments
        .iter()
        .rev()
        .nth(1)
        .copied()
        .and_then(|argument_id| argument_expression_id(ctx, argument_id));

    can_group_expression_argument(ctx, last_id)
        && should_group_last_argument_impl(
            ctx,
            call_node_id,
            arguments.len(),
            penultimate_id,
            last_id,
        )
}

/// Return the grouped call-argument layout, if one standard grouped layout applies.
pub(crate) fn arguments_grouped_layout(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> Option<GroupedCallArgumentLayout> {
    if ctx.has_infix_annotation(call_node_id) || arguments_have_annotations(ctx, arguments) {
        return None;
    }

    if arguments.len() == 2 {
        let first_id = argument_expression_id(ctx, arguments[0])?;
        let second_id = argument_expression_id(ctx, arguments[1])?;

        if can_group_expression_argument(ctx, second_id) {
            return should_group_last_argument(ctx, call_node_id, arguments)
                .then_some(GroupedCallArgumentLayout::GroupedLastArgument);
        }

        return should_group_first_argument(ctx, first_id, second_id)
            .then_some(GroupedCallArgumentLayout::GroupedFirstArgument);
    }

    should_group_last_argument(ctx, call_node_id, arguments)
        .then_some(GroupedCallArgumentLayout::GroupedLastArgument)
}

/// Write one grouped call-argument layout.
pub(crate) fn write_grouped_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_span: destack_source::Span,
    arguments: &[LocalNodeId<Argument>],
    layout: GroupedCallArgumentLayout,
    group_id: GroupId,
    disallow_trailing_separator: bool,
) -> FormatResult<()> {
    let last_index = arguments.len().saturating_sub(1);
    let grouped_index = if layout == GroupedCallArgumentLayout::GroupedFirstArgument {
        0
    } else {
        last_index
    };
    let mut non_grouped_breaks = false;
    let mut grouped_breaks = false;
    let mut elements = Vec::with_capacity(arguments.len());

    // preformat arguments
    for (index, argument_id) in arguments.iter().copied().enumerate() {
        let is_grouped_argument = index == grouped_index;
        let lines_before = if index == 0 {
            0
        } else {
            call_argument_lines_before(f.context(), argument_id)
        };
        let following_span_start = arguments
            .get(index + 1)
            .map(|argument_id| f.context().span(*argument_id).start)
            .unwrap_or(0);
        let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            write_call_argument_in_list(
                f,
                call_span,
                argument_id,
                following_span_start,
                is_grouped_argument.then_some(layout),
                index == 0,
            )
        });
        let interned = f.intern_with_comment_snapshot_after(
            Some(f.context().span(argument_id).start),
            &content,
        )?;

        if interned.as_ref().is_some_and(FirNode::will_break) {
            if is_grouped_argument {
                grouped_breaks = true;
            } else {
                non_grouped_breaks = true;
            }
        }

        elements.push((interned, lines_before));
    }

    // break out every argument once a non-grouped argument forces it
    if non_grouped_breaks {
        return format_all_elements_broken_out(
            f,
            &elements,
            group_id,
            disallow_trailing_separator,
            true,
        );
    }

    let most_flat_elements = elements.clone();
    let format_most_flat = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("(")])?;

        for (index, (element, _)) in most_flat_elements.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }

            if let Some(element) = element.clone() {
                f.write_node(element);
            }
        }

        write!(f, [token(")")])
    });

    let grouped_elements = elements.clone();
    let format_grouped = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token("(")])?;

        let separator = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [token(","), soft_line_break_or_space()])
        });
        let mut joiner = f.join_with(separator);

        for (index, (element, _)) in grouped_elements.iter().enumerate() {
            let Some(element) = element.clone() else {
                continue;
            };
            let entry = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                f.write_node(element.clone());
                Ok(())
            });

            if index == grouped_index {
                joiner.entry(&group(&entry).should_expand(true));
            } else {
                joiner.entry(&entry);
            }
        }

        joiner.finish()?;
        write!(f, [token(")")])
    });

    let format_expanded = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        format_all_elements_broken_out(f, &elements, group_id, disallow_trailing_separator, true)
    });

    if grouped_breaks {
        write!(
            f,
            [best_fitting![format_grouped, format_expanded].with_mode(BestFittingMode::AllLines)]
        )
    } else {
        write!(
            f,
            [
                best_fitting![format_most_flat, format_grouped, format_expanded]
                    .with_mode(BestFittingMode::AllLines)
            ]
        )
    }
}

/// Return whether a call has multiple function-like arguments.
pub(crate) fn is_function_composition_args(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if arguments.len() <= 1 {
        return false;
    }

    let mut has_seen_function_like = false;

    for argument_id in arguments.iter().copied() {
        let Some(expression_id) = argument_expression_id(context, argument_id) else {
            continue;
        };

        if expression_is_function_argument(context, expression_id) {
            if has_seen_function_like {
                return true;
            }

            has_seen_function_like = true;
            continue;
        }

        if let Expression::Call { arguments, .. } = context.tree.get(expression_id) {
            let call_has_function_like_argument = arguments.iter().copied().any(|argument_id| {
                argument_expression_id(context, argument_id).is_some_and(|expression_id| {
                    expression_is_function_argument(context, expression_id)
                })
            });

            if call_has_function_like_argument {
                return true;
            }
        }
    }

    false
}

/// Format precomputed call argument elements in explicit broken-out layout.
fn format_all_elements_broken_out<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[(Option<FirNode>, usize)],
    group_id: GroupId,
    disallow_trailing_separator: bool,
    expand: bool,
) -> FormatResult<()> {
    let write_trailing_separator = !disallow_trailing_separator
        && matches!(
            f.context().options.trailing_comma,
            destack_workspace::TrailingComma::All
        );

    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                for (index, (element, lines_before)) in elements.iter().enumerate() {
                    let Some(element) = element.clone() else {
                        continue;
                    };

                    if index > 0 {
                        match lines_before {
                            0 | 1 => write!(f, [soft_line_break_or_space()])?,
                            _ => write!(f, [empty_line()])?,
                        }
                    }

                    f.write_node(element);

                    if index + 1 != elements.len() {
                        write!(f, [token(",")])?;
                    }
                }

                if write_trailing_separator {
                    write!(f, [token(",")])?;
                }

                Ok(())
            })),
            token(")")
        ])
        .with_id(Some(group_id))
        .should_expand(expand)]
    )
}
