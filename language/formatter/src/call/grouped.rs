use super::argument::{
    argument_enclosing_span, argument_trailing_span, arguments_have_annotations,
    with_argument_following_span_start,
};
use super::list::{call_argument_lines_before, write_call_argument_in_list};
use super::pattern::argument_expression_id;
use crate::annotation::{format_leading_comments, format_trailing_comments};
use crate::chain::{SimpleArgument, transparent_inner_expression};
use crate::declaration::{
    FormatLambdaDeclarationOptions, FunctionCacheMode, GroupedCallArgumentLayout,
    format_function_declaration, format_lambda_declaration_with_options,
};
use crate::{TsppFormatContext, TsppFormatter};
use tspp_dir::{
    Argument, Declaration, Expression, FunctionForm, FunctionSignature, GenericArgument, Literal,
    LocalNodeId, Parameter, TypeExpression, UnaryOperator,
};
use tspp_fir::format::{
    FormatElement as FirElement, FormatError, FormatLayout, FormatResult, GroupId,
};
use tspp_fir::prelude::{
    empty_line, expand_parent, format_with, group, soft_block_indent, soft_line_break_or_space,
    token,
};
use tspp_fir::{best_fitting, format_args, write};
use tspp_repository::TrailingComma;
use tspp_source::{NodeSpanRegion, NodeSpanType};

/// Return whether one expression is a function declaration expression.
fn is_function_argument(
    ctx: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = ctx.tree.get(expression_id) else {
        return false;
    };

    matches!(ctx.tree.get(*declaration_id), Declaration::Function { .. })
}

/// Return whether one lambda declaration can group as one call argument.
fn can_group_lambda_argument(
    context: &TsppFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
    is_lambda_recursion: bool,
) -> bool {
    let Declaration::Function(function) = context.tree.get(declaration_id) else {
        return false;
    };
    let Some(body_id) = function.body else {
        return false;
    };

    // reference return types only group for block bodies
    if let Some(return_type_id) = function.signature.return_type
        && matches!(
            context.tree.get(return_type_id),
            TypeExpression::Reference { .. }
        )
    {
        let body_expression_id = transparent_inner_expression(context, body_id);
        let Expression::Block(block_id) = context.tree.get(body_expression_id) else {
            return false;
        };

        let block = context.tree.get(*block_id);
        let body_span = context
            .tree
            .get_side_span(declaration_id, NodeSpanType::Region(NodeSpanRegion::Body))
            .unwrap_or_else(|| context.span(body_id));

        if block.is_empty() && !context.comments().has_comment_before(body_span.end) {
            return false;
        }
    }

    let body_expression_id = transparent_inner_expression(context, body_id);

    match context.tree.get(body_expression_id) {
        Expression::Block(_)
        | Expression::ObjectExpression { .. }
        | Expression::StructExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::TreeExpression { .. } => true,
        Expression::Declaration(next_declaration_id) => {
            let Declaration::Function(next_function) = context.tree.get(*next_declaration_id)
            else {
                return false;
            };
            if next_function.signature.form != FunctionForm::Lambda {
                return false;
            }

            can_group_lambda_argument(context, *next_declaration_id, true)
        }
        Expression::Call { .. } | Expression::If { .. } => !is_lambda_recursion,
        _ => false,
    }
}

/// Return whether one expression can participate in grouped call-argument layout.
fn can_group_function_argument(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Function(function) = context.tree.get(*declaration_id) else {
        return false;
    };

    if function.signature.form != FunctionForm::Lambda {
        return true;
    }

    can_group_lambda_argument(context, *declaration_id, false)
}

/// Return whether one type expression is simple enough for grouped call layout.
fn is_simple_type_expression(
    ctx: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    let expression_id = extract_array_type_element_expression(ctx, expression_id);
    let expression_id = extract_single_generic_argument_type_expression(ctx, expression_id);

    match ctx.tree.get(expression_id) {
        TypeExpression::Literal { .. }
        | TypeExpression::Keyword { .. }
        | TypeExpression::Intrinsic
        | TypeExpression::Const
        | TypeExpression::This => true,
        TypeExpression::Reference {
            generic_arguments, ..
        }
        | TypeExpression::Member {
            generic_arguments, ..
        } => generic_arguments.is_empty(),
        _ => false,
    }
}

/// Return one type expression after stripping up to two array suffixes.
fn extract_array_type_element_expression(
    ctx: &TsppFormatContext<'_>,
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
fn extract_single_generic_argument_type_expression(
    ctx: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> LocalNodeId<TypeExpression> {
    let generic_arguments = match ctx.tree.get(expression_id) {
        TypeExpression::Reference {
            generic_arguments, ..
        }
        | TypeExpression::Member {
            generic_arguments, ..
        } => generic_arguments,
        _ => return expression_id,
    };

    // keep multi-argument generics intact
    if generic_arguments.len() != 1 {
        return expression_id;
    }

    let (GenericArgument::Type { value }
    | GenericArgument::SpreadType { value }
    | GenericArgument::AssociatedType { value, .. }) = ctx.tree.get(generic_arguments[0])
    else {
        return expression_id;
    };

    *value
}

/// Return whether one expression can participate in grouped call-argument layout.
fn can_group_expression_argument(
    ctx: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(ctx, expression_id);

    match ctx.tree.get(expression_id) {
        Expression::ObjectExpression { properties, .. }
        | Expression::StructExpression { properties, .. } => {
            !properties.is_empty() || ctx.comments().has_comment_in_span(ctx.span(expression_id))
        }
        Expression::ArrayExpression { elements, .. } => {
            !elements.is_empty() || ctx.comments().has_comment_in_span(ctx.span(expression_id))
        }
        Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
            can_group_expression_argument(ctx, transparent_inner_expression(ctx, *expression))
        }
        Expression::Declaration(_) => can_group_function_argument(ctx, expression_id),
        _ => false,
    }
}

/// Return whether one expression is short enough to stay next to a grouped function argument.
fn is_relatively_short_argument(
    ctx: &TsppFormatContext<'_>,
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

            is_simple_type_expression(ctx, right_id)
                && SimpleArgument::from(left_id).is_simple_with_depth(ctx, 1)
        }
        Expression::Call { arguments, .. } => match arguments.len() {
            0 => true,
            1 => SimpleArgument::from(expression_id).is_simple(ctx),
            _ => false,
        },
        Expression::Literal(Literal::RegexString { .. }) => true,
        _ => SimpleArgument::from(expression_id).is_simple(ctx),
    }
}

/// Return whether the first argument should use grouped layout.
fn should_group_first_argument(
    ctx: &TsppFormatContext<'_>,
    first_id: LocalNodeId<Expression>,
    second_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = ctx.tree.get(first_id) else {
        return false;
    };
    let Declaration::Function(function) = ctx.tree.get(*declaration_id) else {
        return false;
    };
    let Some(body_id) = function.body else {
        return false;
    };

    if function.signature.form == FunctionForm::Lambda {
        let body_id = transparent_inner_expression(ctx, body_id);

        if !matches!(ctx.tree.get(body_id), Expression::Block(_)) {
            return false;
        }
    }

    if is_function_argument(ctx, second_id)
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
    ctx: &TsppFormatContext<'_>,
    args_len: usize,
    penultimate_id: Option<LocalNodeId<Expression>>,
    last_id: LocalNodeId<Expression>,
) -> bool {
    if let Some(penultimate_id) = penultimate_id {
        let shares_grouped_layout_kind = matches!(
            (ctx.tree.get(penultimate_id), ctx.tree.get(last_id)),
            (
                Expression::ObjectExpression { .. },
                Expression::ObjectExpression { .. }
            ) | (
                Expression::StructExpression { .. },
                Expression::StructExpression { .. }
            ) | (
                Expression::ArrayExpression { .. },
                Expression::ArrayExpression { .. }
            ) | (Expression::As { .. }, Expression::As { .. })
                | (Expression::Satisfies { .. }, Expression::Satisfies { .. })
        ) || (is_function_argument(ctx, penultimate_id)
            && is_function_argument(ctx, last_id));

        if shares_grouped_layout_kind {
            return false;
        }
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
        comments.has_comment_before(last_span.start)
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
                    is_zero_parameter_block_lambda(ctx, penultimate_id)
                })
            {
                return false;
            }

            !can_concisely_print_array_expression(ctx, last_id)
        }
        _ => true,
    }
}

/// Return whether one expression is a zero-parameter lambda with a block body.
fn is_zero_parameter_block_lambda(
    ctx: &TsppFormatContext<'_>,
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
    function.signature.form == FunctionForm::Lambda
        && function.signature.parameters.is_empty()
        && matches!(ctx.tree.get(body_id), Expression::Block(_))
}

/// Return whether one array expression is concise enough to avoid grouped-last layout.
fn can_concisely_print_array_expression(
    ctx: &TsppFormatContext<'_>,
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
        .all(|argument_id| is_concise_numeric_array_element(ctx, argument_id))
    {
        return false;
    }

    !ctx.comments()
        .comments_before_iter(ctx.span(expression_id).end)
        .any(|comment| comment.is_line() && !comment.preceded_by_newline())
}

/// Return whether one array element is a concise numeric literal element.
fn is_concise_numeric_array_element(
    ctx: &TsppFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(expression_id) = argument_expression_id(ctx, argument_id) else {
        return false;
    };

    is_concise_numeric_literal_expression(ctx, expression_id)
}

/// Return whether one expression is a concise numeric array element.
fn is_concise_numeric_literal_expression(
    ctx: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(ctx, expression_id);

    match ctx.tree.get(expression_id) {
        Expression::Literal(Literal::Integer(_) | Literal::Float(_)) => true,
        Expression::Unary { operator, right } => {
            matches!(
                operator,
                UnaryOperator::Plus
                    | UnaryOperator::Negate
                    | UnaryOperator::Not
                    | UnaryOperator::ElementwiseNot
            ) && matches!(
                ctx.tree.get(transparent_inner_expression(ctx, *right)),
                Expression::Literal(Literal::Integer(_) | Literal::Float(_))
            ) && !ctx.comments().has_comment_in_span(ctx.span(expression_id))
        }
        _ => false,
    }
}

/// Return whether the last argument should use grouped layout.
fn should_group_last_argument(
    ctx: &TsppFormatContext<'_>,
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
        && should_group_last_argument_impl(ctx, arguments.len(), penultimate_id, last_id)
}

/// Return the grouped call-argument layout, if one standard grouped layout applies.
pub(crate) fn arguments_grouped_layout(
    ctx: &TsppFormatContext<'_>,
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
            return should_group_last_argument(ctx, arguments)
                .then_some(GroupedCallArgumentLayout::GroupedLastArgument);
        }

        return should_group_first_argument(ctx, first_id, second_id)
            .then_some(GroupedCallArgumentLayout::GroupedFirstArgument);
    }

    should_group_last_argument(ctx, arguments)
        .then_some(GroupedCallArgumentLayout::GroupedLastArgument)
}

/// Return whether one parameter list stays simple enough for grouped function arguments.
fn grouped_function_signature_is_simple(
    context: &TsppFormatContext<'_>,
    signature: &FunctionSignature,
) -> bool {
    if signature.this_parameter.is_some() {
        return false;
    }

    signature.parameters.iter().copied().all(|parameter_id| {
        matches!(
            context.tree.get(parameter_id),
            Parameter::Named {
                declared_type,
                default,
                ..
            } if default.is_none() && declared_type.is_none()
        )
    })
}

/// Return the function declaration eligible for grouped call formatting.
fn grouped_function_argument_declaration_id(
    context: &TsppFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    layout: GroupedCallArgumentLayout,
    is_only_argument: bool,
) -> Option<LocalNodeId<Declaration>> {
    let value_id =
        transparent_inner_expression(context, argument_expression_id(context, argument_id)?);
    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return None;
    };
    let Declaration::Function(function) = context.tree.get(*declaration_id) else {
        return None;
    };

    if layout == GroupedCallArgumentLayout::GroupedFirstArgument {
        return (function.signature.form == FunctionForm::Lambda).then_some(*declaration_id);
    }

    if function.signature.form == FunctionForm::Lambda
        || (!is_only_argument && grouped_function_signature_is_simple(context, &function.signature))
    {
        return Some(*declaration_id);
    }

    None
}

/// Write one function argument through its declaration owner.
fn write_function_argument_with_options<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    declaration_id: LocalNodeId<Declaration>,
    following_span_start: Option<u32>,
    call_argument_layout: Option<GroupedCallArgumentLayout>,
    cache_mode: FunctionCacheMode,
) -> FormatResult<()> {
    let Declaration::Function(function) = f.context().tree.get(declaration_id) else {
        return Err(FormatError::SyntaxError {
            message: "grouped function argument requires a function declaration",
        });
    };

    let argument = f.context().tree.get(argument_id);
    let argument_span = f.context().span(argument_id);
    let trailing_span = argument_trailing_span(f.context(), argument_id);
    let enclosing_span = argument_enclosing_span(f.context(), argument_id)?;

    // leading comments
    write!(f, [format_leading_comments(argument_span)])?;

    // payload
    match argument {
        Argument::Spread { .. } => {
            write!(f, [token("...")])?;
        }
        Argument::Positional { .. } | Argument::Elision | Argument::Error => {}
    }

    // declaration
    match function.signature.form {
        FunctionForm::Lambda => {
            let options = FormatLambdaDeclarationOptions {
                assignment_layout: None,
                call_argument_layout,
                cache_mode,
            };

            format_lambda_declaration_with_options(
                f,
                declaration_id,
                function.export,
                function.is_ambient,
                function.name,
                &function.signature,
                &function.body,
                options,
            )?;
        }
        FunctionForm::Function => {
            format_function_declaration(
                f,
                declaration_id,
                function.export,
                function.is_ambient,
                function.name,
                &function.signature,
                &function.body,
                cache_mode,
            )?;
        }
    }

    // trailing comments
    write!(
        f,
        [format_trailing_comments(
            enclosing_span,
            trailing_span,
            following_span_start,
        )]
    )
}

/// Write one grouped argument replacement entry.
fn write_grouped_argument_entry<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    following_span_start: Option<u32>,
    write_comma: bool,
    layout: GroupedCallArgumentLayout,
    is_only_argument: bool,
) -> FormatResult<()> {
    let Some(declaration_id) = grouped_function_argument_declaration_id(
        f.context(),
        argument_id,
        layout,
        is_only_argument,
    ) else {
        let separator = if write_comma {
            super::list::CallArgumentSeparator::Always
        } else {
            super::list::CallArgumentSeparator::None
        };

        return write_call_argument_in_list(f, argument_id, following_span_start, separator);
    };

    with_argument_following_span_start(f, following_span_start, |f| {
        write_function_argument_with_options(
            f,
            argument_id,
            declaration_id,
            following_span_start,
            Some(layout),
            FunctionCacheMode::Cache,
        )
    })?;

    if write_comma {
        write!(f, [token(",")])?;
    }

    Ok(())
}

/// Write one grouped call-argument layout.
pub(crate) fn write_grouped_arguments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
    let following_span_starts = arguments
        .iter()
        .enumerate()
        .map(|(index, _)| {
            arguments
                .get(index + 1)
                .map(|argument_id| f.context().span(*argument_id).start)
        })
        .collect::<Vec<_>>();
    let mut non_grouped_breaks = false;
    let mut grouped_breaks = false;
    let mut has_cached = false;
    let mut elements = Vec::with_capacity(arguments.len());

    // preformat entries
    for (index, argument_id) in arguments.iter().copied().enumerate() {
        let is_grouped_argument = index == grouped_index;
        let following_span_start = following_span_starts[index];
        let lines_before = if index == 0 {
            0
        } else {
            call_argument_lines_before(f.context(), argument_id)
        };

        let content = format_with(move |f: &mut TsppFormatter<'ast, '_>| {
            let is_only_argument = index == 0 && last_index == 0;

            if is_grouped_argument {
                let declaration_id = grouped_function_argument_declaration_id(
                    f.context(),
                    argument_id,
                    layout,
                    is_only_argument,
                );

                if let Some(declaration_id) = declaration_id {
                    write_function_argument_with_options(
                        f,
                        argument_id,
                        declaration_id,
                        following_span_start,
                        None,
                        FunctionCacheMode::Cache,
                    )?;

                    if index != last_index {
                        write!(f, [token(",")])?;
                    }

                    return Ok(());
                }
            }

            let separator = if index != last_index {
                super::list::CallArgumentSeparator::Always
            } else {
                super::list::CallArgumentSeparator::None
            };

            write_call_argument_in_list(f, argument_id, following_span_start, separator)
        });
        let element = f.capture(&content)?;

        if is_grouped_argument {
            let is_only_argument = index == 0 && last_index == 0;
            let is_cached_argument = grouped_function_argument_declaration_id(
                f.context(),
                argument_id,
                layout,
                is_only_argument,
            )
            .is_some();

            has_cached |= is_cached_argument;
            grouped_breaks |= element.as_ref().is_some_and(FirElement::will_break);
        } else {
            non_grouped_breaks |= element.as_ref().is_some_and(FirElement::will_break);
        }

        elements.push((element, lines_before));
    }

    if non_grouped_breaks {
        return format_all_elements_broken_out(
            f,
            &elements,
            group_id,
            disallow_trailing_separator,
            true,
        );
    }

    if has_cached {
        let argument_id = arguments[grouped_index];
        let is_only_argument = arguments.len() == 1;
        let declaration_id = grouped_function_argument_declaration_id(
            f.context(),
            argument_id,
            layout,
            is_only_argument,
        )
        .ok_or(FormatError::SyntaxError {
            message: "cached grouped argument requires an eligible function declaration",
        })?;
        let cache_key = f
            .context()
            .tree
            .get_side_span(
                declaration_id,
                NodeSpanType::Region(NodeSpanRegion::Parameters),
            )
            .ok_or(FormatError::SyntaxError {
                message: "grouped function argument requires a parameter span",
            })?;
        let cached_signature =
            f.context()
                .cached_element(&cache_key)
                .ok_or(FormatError::SyntaxError {
                    message: "grouped function argument requires a cached signature",
                })?;
        let element = cached_signature.remove_soft_lines(f.allocator());

        if element.as_ref().is_some_and(FirElement::will_break) {
            return format_all_elements_broken_out(
                f,
                &elements,
                group_id,
                disallow_trailing_separator,
                true,
            );
        }
        if let Some(element) = element {
            f.context_mut().cache_element(&cache_key, element);
        }

        let content = format_with(move |f: &mut TsppFormatter<'ast, '_>| {
            write_grouped_argument_entry(
                f,
                argument_id,
                following_span_starts[grouped_index],
                grouped_index != last_index,
                layout,
                arguments.len() == 1,
            )
        });
        let element = f.capture(&content)?;
        elements[grouped_index].0 = element;
    }

    let format_most_flat = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        write!(f, [token("(")])?;

        let separator = soft_line_break_or_space();
        let mut joiner = f.join_with(&separator);

        for (element, _) in &elements {
            let Some(element) = *element else {
                continue;
            };

            joiner.entry(&format_with(move |f: &mut TsppFormatter<'ast, '_>| {
                f.write_element(element);
                Ok(())
            }));
        }

        joiner.finish()?;

        write!(f, [token(")")])
    });

    let format_grouped = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        write!(f, [token("(")])?;

        let separator = soft_line_break_or_space();
        let mut joiner = f.join_with(separator);

        for (index, (element, _)) in elements.iter().enumerate() {
            let Some(element) = *element else {
                continue;
            };

            let entry = format_with(move |f: &mut TsppFormatter<'ast, '_>| {
                f.write_element(element);
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

    let format_expanded = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        format_all_elements_broken_out(f, &elements, group_id, disallow_trailing_separator, true)
    });

    if grouped_breaks {
        write!(f, [expand_parent()])?;
        write!(f, [best_fitting![format_grouped, format_expanded]])?;
    } else {
        write!(
            f,
            [best_fitting![
                format_most_flat,
                format_grouped,
                format_expanded
            ]]
        )?;
    }

    Ok(())
}

/// Return whether a call has multiple function-like arguments.
pub(crate) fn is_function_composition_args(
    context: &TsppFormatContext<'_>,
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

        if is_function_argument(context, expression_id) {
            if has_seen_function_like {
                return true;
            }

            has_seen_function_like = true;
            continue;
        }

        if let Expression::Call { arguments, .. } = context.tree.get(expression_id) {
            let call_has_function_like_argument = arguments.iter().copied().any(|argument_id| {
                argument_expression_id(context, argument_id)
                    .is_some_and(|expression_id| is_function_argument(context, expression_id))
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
    f: &mut TsppFormatter<'ast, '_>,
    elements: &[(Option<FirElement<'ast>>, usize)],
    group_id: GroupId,
    disallow_trailing_separator: bool,
    expand: bool,
) -> FormatResult<()> {
    let write_trailing_separator = !disallow_trailing_separator
        && matches!(f.context().options.trailing_comma, TrailingComma::All);

    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&format_with(move |f: &mut TsppFormatter<'ast, '_>| {
                for (index, (element, lines_before)) in elements.iter().enumerate() {
                    if index > 0 {
                        match lines_before {
                            0 | 1 => write!(f, [soft_line_break_or_space()])?,
                            _ => write!(f, [empty_line()])?,
                        }
                    }

                    let Some(element) = *element else {
                        continue;
                    };

                    f.write_element(element);
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
