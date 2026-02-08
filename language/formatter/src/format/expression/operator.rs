use super::*;
use destack_fir::{format_args, write};

/// Return whether a leading type union has an expression ancestor with block-prefix comments.
fn leading_union_has_ancestor_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;
    while let Some((parent_id, parent_type)) = context.get_parent(current_id) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        if let Some(annotation_ids) = context.get_annotations(parent_expression_id)
            && annotation_ids.iter().any(|annotation_id| {
                let annotation = context.tree.get::<Annotation>(*annotation_id);
                if !matches!(annotation.position(), AnnotationPosition::BlockPrefix)
                    || !matches!(annotation, Annotation::Comment { .. })
                {
                    return false;
                }

                let annotation_span = context.get_span::<Annotation>(*annotation_id);
                let annotation_source = context.get_span_str(annotation_span);
                let trimmed = annotation_source.trim_start();
                annotation_source.contains('\n') && !trimmed.starts_with("/**")
            })
        {
            return true;
        }

        current_id = parent_expression_id;
    }

    false
}

/// Format operator and chain expression variants.
pub(super) fn format_operator_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    let tree = f.context().tree;

    match expression {
        // unary
        Expression::Unary { operator, right } => {
            if operator.is_prefix() {
                let needs_space = matches!(operator, UnaryOperator::Typeof | UnaryOperator::Void);
                if needs_space {
                    write!(f, [operator, space(), right])?;
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
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_member_expression(f, node_id)?;
            }
        }

        // index
        Expression::Index { left, .. } => {
            let left_is_instantiation = matches!(tree.get(*left), Expression::Instantiation { .. });
            if is_expression_chain(tree, node_id) && !left_is_instantiation {
                format_expression_chain(f, node_id)?;
            } else {
                format_index_expression(f, node_id)?;
            }
        }

        // call
        Expression::Call { .. } => {
            if is_expression_chain(tree, node_id) || call_prefers_chain_format(f.context(), node_id)
            {
                format_expression_chain(f, node_id)?;
            } else {
                format_call_expression(f, node_id)?;
            }
        }

        // instantiation
        Expression::Instantiation { .. } => {
            if is_expression_chain(tree, node_id) && has_chain_parent(f.context(), node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_instantiation_expression(f, node_id)?;
            }
        }

        // new
        Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            let mut left = *left;
            if let Expression::Parenthesized { expression } = tree.get(left)
                && should_unwrap_parenthesized_new_member_callee(f.context(), left, *expression)
            {
                left = *expression;
            }

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
                    } if member_object_prefers_new_callee_parentheses(f.context(), *member_left)
                );
                if should_wrap_member_callee {
                    write!(f, [token("new"), space(), token("("), left, token(")")])?;
                } else {
                    write!(f, [token("new"), space(), left])?;
                }
            }
            if let Some(static_arguments) = static_arguments {
                format_static_argument_list(f, static_arguments)?;
            }
            format_call_arguments(f, node_id, dynamic_arguments)?;
        }

        // delete
        Expression::Delete { value } => {
            write!(f, [token("delete"), space(), value])?;
        }

        // maybe
        Expression::Maybe { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_maybe_expression(f, node_id)?;
            }
        }

        // must
        Expression::Must { position, left } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                let needs_parentheses = needs_parens_in_postfix_position(tree, *left);
                if needs_parentheses {
                    write!(f, [token("("), left, token(")")])?;
                } else {
                    write!(f, [left])?;
                }
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                write!(f, [token("!")])?;
            }
        }

        // binary
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let in_type_context = is_type_context(f.context(), node_id);
            let is_destack = f.context().options.language_type.is_destack();
            let is_type_intersection =
                in_type_context && *operator == BinaryOperator::ElementwiseAnd;
            let is_type_union = in_type_context && *operator == BinaryOperator::ElementwiseOr;

            if *operator == BinaryOperator::Coalesce
                && should_use_trailing_coalesce(f.context(), node_id, *left)
            {
                let has_postfix = f.context().has_postfix_annotation(*left);
                write!(
                    f,
                    [group(&format_args![
                        left,
                        indent(&format_with(|f| {
                            if !has_postfix {
                                write!(f, [space()])?;
                            }
                            write!(f, [operator, soft_line_break_or_space(), right])
                        }))
                    ])]
                )?;
                return Ok(true);
            }

            // preserve operator trailing line comments:
            // `left || // comment` then rhs on the next line
            if matches!(
                operator,
                BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
            ) && let Some(line_comment) =
                line_comment_between_expressions(f.context(), *left, *right)
            {
                let has_postfix = f.context().has_postfix_annotation(*left);
                let right_without_prefix = format_with(|f| {
                    let right_directive = directive_for_node(f.context(), *right);
                    format_expression(f, *right, f.context().tree.get(*right), right_directive)?;
                    if !matches!(
                        right_directive,
                        Some(FormatterDirective {
                            kind: FormatterDirectiveKind::IgnoreFormat,
                            position: FormatterDirectivePosition::Postfix { .. },
                        })
                    ) {
                        write!(f, [f.context().any_infix_or_postfix_annotations(*right)])?;
                    }
                    Ok(())
                });
                write!(
                    f,
                    [group(&format_args![
                        left,
                        format_with(|f| {
                            if !has_postfix {
                                write!(f, [space()])?;
                            }
                            Ok(())
                        }),
                        operator,
                        space(),
                        text(line_comment.as_str()),
                        indent(&format_args![hard_line_break(), right_without_prefix])
                    ])]
                )?;
                return Ok(true);
            }

            // keep precedence mixed logical rhs grouped for readability:
            // `a || b && c` -> `a || (b && c)`
            if matches!(operator, BinaryOperator::Or | BinaryOperator::Coalesce)
                && let Expression::Binary {
                    operator: right_operator,
                    ..
                } = f.context().tree.get(*right)
                && *right_operator != *operator
                && matches!(
                    right_operator,
                    BinaryOperator::And | BinaryOperator::Coalesce
                )
            {
                let has_postfix = f.context().has_postfix_annotation(*left);
                write!(
                    f,
                    [group(&format_args![
                        left,
                        format_with(|f| {
                            if !has_postfix {
                                write!(f, [space()])?;
                            }
                            Ok(())
                        }),
                        operator,
                        space(),
                        token("("),
                        right,
                        token(")")
                    ])]
                )?;
                return Ok(true);
            }

            // keep `left && (` on the same line for jsx parents when it fits
            if matches!(
                operator,
                BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
            ) {
                let left_span = f.context().get_span(*left);
                let left_source = f.context().get_span_str(left_span);
                let left_has_multiline_parenthesized_tail =
                    f.context().has_newline(left_span) && left_source.trim_end().ends_with(')');
                let right_expression = f.context().tree.get(*right);
                let right_is_short_trivial =
                    is_trivial_expression(f.context().tree, right_expression)
                        && expression_source_len(f.context(), *right)
                            <= usize::from(f.context().options.line_width) / 4;
                let right_has_prefix = f.context().has_prefix_annotation(*right);
                if left_has_multiline_parenthesized_tail
                    && right_is_short_trivial
                    && !right_has_prefix
                    && *operator == BinaryOperator::And
                {
                    let has_postfix = f.context().has_postfix_annotation(*left);
                    write!(
                        f,
                        [group(&format_args![
                            left,
                            format_with(|f| {
                                if !has_postfix {
                                    write!(f, [space()])?;
                                }
                                Ok(())
                            }),
                            operator,
                            indent(&format_args![hard_line_break(), right])
                        ])]
                    )?;
                    return Ok(true);
                }

                let left_prefers_trailing_operator = matches!(
                    f.context().tree.get(*left),
                    Expression::Parenthesized { expression }
                        if (f.context().has_prefix_annotation(*left)
                            || f.context().has_prefix_annotation(*expression))
                            && f.context().has_newline(f.context().get_span(*left))
                );
                if left_prefers_trailing_operator {
                    let right_len = expression_source_len(f.context(), *right);
                    let trailing_threshold = usize::from(f.context().options.line_width) / 4;
                    if right_len <= trailing_threshold {
                        let has_postfix = f.context().has_postfix_annotation(*left);
                        write!(
                            f,
                            [group(&format_args![
                                left,
                                format_with(|f| {
                                    if !has_postfix {
                                        write!(f, [space()])?;
                                    }
                                    Ok(())
                                }),
                                operator,
                                space(),
                                right
                            ])]
                        )?;
                        return Ok(true);
                    }
                }

                let is_parenthesized_tree = matches!(
                    right_expression,
                    Expression::Parenthesized { expression }
                        if matches!(f.context().tree.get(*expression), Expression::TreeExpression { .. })
                );

                if is_parenthesized_tree {
                    let line_width = usize::from(f.context().options.line_width);
                    let remaining_width =
                        assignment_like_remaining_width(f.context(), node_id).unwrap_or(line_width);
                    let left_len = expression_source_len(f.context(), *left);
                    let operator_len = binary_operator_len(operator);
                    let inline_len = left_len.saturating_add(operator_len).saturating_add(3);

                    if inline_len <= remaining_width {
                        write!(
                            f,
                            [group(&format_args![
                                left,
                                space(),
                                operator,
                                space(),
                                right
                            ])]
                        )?;
                        return Ok(true);
                    }
                }
            }

            // flatten binary expression chain for Prettier-style formatting
            // e.g. `a + b + c` formats as:
            //   a
            //       + b
            //       + c
            // all operands at the same indentation level
            let operands = if is_type_union || is_type_intersection {
                flatten_type_binary_expression(f.context(), node_id, *operator)
            } else {
                flatten_binary_expression(f.context().tree, node_id, *operator)
            };

            // preserve leading `|` formatting when present in ts union source
            if is_type_union && union_source_has_leading_pipe(f.context(), node_id) {
                let has_block_prefix_ancestor =
                    leading_union_has_ancestor_block_prefix_annotation(f.context(), node_id);
                let should_indent_leading_pipe_operands =
                    !f.context().has_prefix_annotation(node_id) && !has_block_prefix_ancestor;
                let prefer_space_before_first_leading_pipe = has_block_prefix_ancestor;
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                        for (index, operand) in operands.iter().enumerate() {
                            if index == 0 {
                                if should_indent_leading_pipe_operands {
                                    write!(
                                        f,
                                        [indent(&format_with(|f| {
                                            if prefer_space_before_first_leading_pipe {
                                                write!(f, [token("|"), space(), operand.expression])
                                            } else {
                                                write!(
                                                    f,
                                                    [
                                                        soft_line_break_or_space(),
                                                        token("|"),
                                                        space(),
                                                        operand.expression
                                                    ]
                                                )
                                            }
                                        }))]
                                    )?;
                                } else if prefer_space_before_first_leading_pipe {
                                    write!(f, [token("|"), space(), operand.expression])?;
                                } else {
                                    write!(
                                        f,
                                        [
                                            soft_line_break_or_space(),
                                            token("|"),
                                            space(),
                                            operand.expression
                                        ]
                                    )?;
                                }
                            } else {
                                let has_postfix = prev_expression.is_some_and(|expression_id| {
                                    f.context().has_postfix_annotation(expression_id)
                                });
                                let between_line_comment =
                                    prev_expression.and_then(|expression_id| {
                                        line_comment_between_expressions(
                                            f.context(),
                                            expression_id,
                                            operand.expression,
                                        )
                                    });
                                let operand_without_prefix = format_with(|f| {
                                    let operand_expression =
                                        normalize_type_binary_operand_expression(
                                            f.context(),
                                            operand.expression,
                                            *operator,
                                        );
                                    let directive =
                                        directive_for_node(f.context(), operand_expression);
                                    format_expression(
                                        f,
                                        operand_expression,
                                        f.context().tree.get(operand_expression),
                                        directive,
                                    )?;
                                    if !matches!(
                                        directive,
                                        Some(FormatterDirective {
                                            kind: FormatterDirectiveKind::IgnoreFormat,
                                            position: FormatterDirectivePosition::Postfix { .. },
                                        })
                                    ) {
                                        write!(
                                            f,
                                            [f.context().any_infix_or_postfix_annotations(
                                                operand_expression
                                            )]
                                        )?;
                                    }
                                    Ok(())
                                });
                                if should_indent_leading_pipe_operands {
                                    write!(
                                        f,
                                        [indent(&format_with(|f| {
                                            if !has_postfix {
                                                write!(f, [soft_line_break_or_space()])?;
                                            }
                                            if let Some(line_comment) =
                                                between_line_comment.as_ref()
                                            {
                                                write!(
                                                    f,
                                                    [
                                                        text(line_comment.as_str()),
                                                        hard_line_break(),
                                                        token("|"),
                                                        space(),
                                                        operand_without_prefix
                                                    ]
                                                )
                                            } else {
                                                write!(
                                                    f,
                                                    [token("|"), space(), operand_without_prefix]
                                                )
                                            }
                                        }))]
                                    )?;
                                } else {
                                    if !has_postfix {
                                        write!(f, [soft_line_break_or_space()])?;
                                    }
                                    if let Some(line_comment) = between_line_comment.as_ref() {
                                        write!(
                                            f,
                                            [
                                                text(line_comment.as_str()),
                                                hard_line_break(),
                                                token("|"),
                                                space(),
                                                operand_without_prefix
                                            ]
                                        )?;
                                    } else {
                                        write!(f, [token("|"), space(), operand_without_prefix])?;
                                    }
                                }
                            }
                            prev_expression = Some(operand.expression);
                        }

                        Ok(())
                    }))
                    .should_expand(true)]
                )?;
                return Ok(true);
            }

            // nullable union types stay inline with `|` separators
            if is_type_union && should_hug_nullable_union_type(f.context(), &operands) {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                        for operand in &operands {
                            if let Some(op) = operand.operator {
                                let has_postfix = prev_expression
                                    .is_some_and(|e| f.context().has_postfix_annotation(e));
                                if !has_postfix {
                                    write!(f, [space()])?;
                                }
                                write!(f, [op, space(), operand.expression])?;
                            } else {
                                write!(f, [operand.expression])?;
                            }
                            prev_expression = Some(operand.expression);
                        }
                        Ok(())
                    }))]
                )?;
                return Ok(true);
            }

            // keep static argument unions compact to match generic hugging behavior
            if is_type_union
                && should_hug_static_argument_union_type(f.context(), node_id, &operands)
            {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                        for operand in &operands {
                            if let Some(op) = operand.operator {
                                let has_postfix = prev_expression.is_some_and(|expression_id| {
                                    f.context().has_postfix_annotation(expression_id)
                                });
                                if !has_postfix {
                                    write!(f, [space()])?;
                                }
                                write!(f, [op, space(), operand.expression])?;
                            } else {
                                write!(f, [operand.expression])?;
                            }
                            prev_expression = Some(operand.expression);
                        }
                        Ok(())
                    }))]
                )?;
                return Ok(true);
            }

            // destack intersections: prefer trailing `&` with object-like heuristics
            if is_type_intersection && is_destack {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                        let mut prev_object_like = false;
                        let mut prev_has_annotation = false;

                        for operand in &operands {
                            if let Some(op) = operand.operator {
                                let has_postfix = prev_expression
                                    .is_some_and(|e| f.context().has_postfix_annotation(e));
                                let is_object_like =
                                    is_object_like_type_expression(f.context(), operand.expression);
                                let current_has_annotation =
                                    f.context().has_annotation(operand.expression);
                                let allow_break = !(prev_object_like || is_object_like)
                                    || prev_has_annotation
                                    || current_has_annotation;

                                if !has_postfix {
                                    write!(f, [space()])?;
                                }

                                write!(f, [op])?;

                                if allow_break {
                                    write!(
                                        f,
                                        [indent(&format_with(
                                            |f: &mut DestackFormatter<'ast, '_>| {
                                                write!(
                                                    f,
                                                    [
                                                        soft_line_break_or_space(),
                                                        operand.expression
                                                    ]
                                                )
                                            }
                                        ))]
                                    )?;
                                } else {
                                    write!(f, [space(), operand.expression])?;
                                }

                                prev_object_like = is_object_like;
                                prev_has_annotation = current_has_annotation;
                            } else {
                                write!(f, [operand.expression])?;
                                prev_object_like =
                                    is_object_like_type_expression(f.context(), operand.expression);
                                prev_has_annotation =
                                    f.context().has_annotation(operand.expression);
                            }

                            prev_expression = Some(operand.expression);
                        }

                        Ok(())
                    }))]
                )?;
                return Ok(true);
            }

            // default: leading operator on break
            let binary_parent_is_parenthesized =
                f.context()
                    .get_parent(node_id)
                    .is_some_and(|(parent_id, parent_type)| {
                        parent_type == NodeType::Expression
                            && matches!(
                                f.context()
                                    .tree
                                    .get(LocalNodeId::<Expression>::new(parent_id)),
                                Expression::Parenthesized { expression } if *expression == node_id
                            )
                    });
            write!(
                f,
                [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                    for operand in &operands {
                        if let Some(op) = operand.operator {
                            // check if previous operand has postfix annotation (it adds its own space)
                            let has_postfix = prev_expression
                                .is_some_and(|e| f.context().has_postfix_annotation(e));
                            let previous_has_prefix_annotation =
                                prev_expression.is_some_and(|expression_id| {
                                    expression_has_leading_prefix_comment(
                                        f.context(),
                                        expression_id,
                                    )
                                });
                            let operand_prefers_trailing_operator = matches!(
                                op,
                                BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
                            ) && (f
                                .context()
                                .has_prefix_annotation(operand.expression)
                                || f.context()
                                    .has_newline(f.context().get_span(operand.expression)));
                            if operand_prefers_trailing_operator {
                                if !has_postfix {
                                    write!(f, [space()])?;
                                }
                                write!(
                                    f,
                                    [
                                        op,
                                        indent(&format_args![
                                            hard_line_break(),
                                            format_with(|f| {
                                                format_binary_operand_with_grouping_parentheses(
                                                    f,
                                                    *operator,
                                                    operand.expression,
                                                )
                                            })
                                        ])
                                    ]
                                )?;
                            } else if previous_has_prefix_annotation {
                                if !has_postfix {
                                    write!(f, [space()])?;
                                }
                                write!(f, [op, space()])?;
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    *operator,
                                    operand.expression,
                                )?;
                            } else {
                                // subsequent operands: soft break, operator, space, operand
                                write!(
                                    f,
                                    [indent(&format_with(
                                        |f: &mut DestackFormatter<'ast, '_>| {
                                            if !has_postfix {
                                                write!(f, [soft_line_break_or_space()])?;
                                            }
                                            write!(f, [op, space()])?;
                                            format_binary_operand_with_grouping_parentheses(
                                                f,
                                                *operator,
                                                operand.expression,
                                            )
                                        }
                                    ))]
                                )?;
                            }
                        } else {
                            // first operand has no preceding operator
                            let first_operand_has_prefix_annotation =
                                f.context().has_prefix_annotation(operand.expression);
                            let should_indent_first_operand = first_operand_has_prefix_annotation
                                && (binary_parent_is_parenthesized
                                    || is_type_union
                                    || is_type_intersection);
                            if should_indent_first_operand {
                                write!(
                                    f,
                                    [indent(&format_args![format_with(|f| {
                                        format_binary_operand_with_grouping_parentheses(
                                            f,
                                            *operator,
                                            operand.expression,
                                        )
                                    })])]
                                )?;
                            } else {
                                format_binary_operand_with_grouping_parentheses(
                                    f,
                                    *operator,
                                    operand.expression,
                                )?;
                            }
                        }
                        prev_expression = Some(operand.expression);
                    }
                    Ok(())
                }))]
            )?;
        }

        // type binary
        Expression::TypeBinary {
            left,
            operator,
            right,
        } => {
            let mut formatted_left = *left;
            if matches!(
                operator,
                TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
            ) && let Expression::Parenthesized { expression } = f.context().tree.get(*left)
                && (!parenthesized_has_leading_inner_trivia(f.context(), *left, *expression)
                    || matches!(
                        f.context().tree.get(*expression),
                        Expression::TypeBinary {
                            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
                            ..
                        }
                    ))
                && !f.context().has_annotation(*left)
                && should_drop_type_binary_left_parentheses(f.context(), node_id, *expression)
            {
                formatted_left = *expression;
            }

            let has_postfix = f.context().has_postfix_annotation(formatted_left);
            let left_has_leading_prefix_comment =
                expression_has_leading_prefix_comment(f.context(), formatted_left);
            let left_is_chain_expression = is_expression_chain(f.context().tree, formatted_left)
                || is_chain_root(f.context().tree, formatted_left);
            let line_width = usize::from(f.context().options.line_width);
            let is_parenthesized_new_callee =
                type_binary_is_parenthesized_new_callee(f.context(), node_id);
            let should_expand_chain_left = matches!(
                operator,
                TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
            ) && left_is_chain_expression
                && (expression_source_len(f.context(), node_id) > line_width
                    || is_parenthesized_new_callee);

            if should_expand_chain_left {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [formatted_left])
                            }))
                            .should_expand(true)]
                        )?;
                        if !has_postfix {
                            write!(f, [space()])?;
                        }
                        write!(f, [operator, space(), right])
                    }))]
                )?;
            } else {
                write!(
                    f,
                    [group(&format_args![
                        formatted_left,
                        indent(&format_with(|f| {
                            if !has_postfix {
                                if left_has_leading_prefix_comment {
                                    write!(f, [space()])?;
                                } else {
                                    write!(f, [soft_line_break_or_space()])?;
                                }
                            }
                            write!(f, [operator, space(), right])
                        }))
                    ])]
                )?;
            }
        }

        // assign
        Expression::Assign {
            left,
            operator,
            right,
        } => {
            let has_postfix = f.context().has_postfix_annotation(*left);
            let inner_right_id = transparent_inner_expression(f.context(), *right);
            let inner_right_expr = f.context().tree.get(inner_right_id);

            // binaries, chains, and nested lambda tails handle their own breaking
            let right_is_binary = matches!(inner_right_expr, Expression::Binary { .. });
            let right_is_sequence =
                matches!(inner_right_expr, Expression::SequenceExpression { .. });
            let right_is_chain_root = is_chain_root(f.context().tree, inner_right_id);
            let right_is_chain =
                is_expression_chain(f.context().tree, inner_right_id) || right_is_chain_root;
            let right_is_chain_tail_lambda =
                is_assignment_chain_tail_lambda(f.context(), node_id, inner_right_id);
            let right_is_lambda = is_lambda_expression(f.context(), inner_right_id);
            let right_handles_its_own_breaking = right_is_binary
                || right_is_sequence
                || right_is_chain
                || right_is_chain_tail_lambda
                || right_is_lambda;
            let right_has_prefix_annotation = f.context().has_prefix_annotation(*right);
            let right_has_newline = f.context().has_newline(f.context().get_span(*right));
            let right_has_existing_operator_break =
                has_newline_between_expressions(f.context(), *left, *right);
            let right_has_between_comment =
                has_comment_between_expressions(f.context(), *left, *right);

            // string literals are atomic: never break at `=`
            let is_string_literal = matches!(
                inner_right_expr,
                Expression::ScalarLiteral(ScalarLiteral::String(_))
                    | Expression::TemplateExpression { .. }
            );

            // estimate remaining inline width for rhs
            let line_width = usize::from(f.context().options.line_width);
            let left_source_len = expression_source_len(f.context(), *left);
            let operator_len = assign_operator_len(operator);
            let inline_overhead = left_source_len
                .saturating_add(operator_len)
                .saturating_add(2);
            let remaining_width = line_width.saturating_sub(inline_overhead);
            let right_source_len = expression_source_len(f.context(), inner_right_id);
            let right_annotation_len = expression_prefix_annotation_source_len(f.context(), *right);
            let right_source_len = right_source_len.saturating_add(right_annotation_len);
            let right_is_long = right_source_len > remaining_width;

            // prefer breaking after `=` for long binary rhs values
            let right_is_long_binary = if right_is_binary {
                let binary_operand_count = match inner_right_expr {
                    Expression::Binary { operator, .. } => {
                        flatten_binary_expression(f.context().tree, inner_right_id, *operator).len()
                    }
                    _ => 0,
                };
                let is_very_long_binary = right_source_len > line_width.saturating_sub(4);

                right_source_len > remaining_width
                    && is_very_long_binary
                    && binary_operand_count > 2
            } else {
                false
            };

            if is_string_literal {
                write!(
                    f,
                    [group(&format_args![
                        left,
                        format_with(|f| {
                            if !has_postfix {
                                write!(f, [space()])?;
                            }
                            Ok(())
                        }),
                        operator,
                        space(),
                        right
                    ])]
                )?;
                return Ok(true);
            }

            // format the space before the operator, respecting postfix comments
            let space_before_operator = format_with(|f| {
                if !has_postfix {
                    write!(f, [space()])?;
                }
                Ok(())
            });

            // break long binary rhs values after the operator
            if right_is_long_binary {
                let format_break_after_operator = format_with(|f| {
                    // avoid double indentation when the rhs breaks internally
                    let dedented_right = dedent(&right);
                    write!(
                        f,
                        [
                            left,
                            space_before_operator,
                            operator,
                            indent(&format_args![hard_line_break(), dedented_right])
                        ]
                    )
                });

                let format_inline = format_with(|f| {
                    group(&format_args![
                        left,
                        space_before_operator,
                        operator,
                        space(),
                        right
                    ])
                    .format(f)
                });

                best_fitting![format_inline, format_break_after_operator]
                    .with_mode(BestFittingMode::AllLines)
                    .format(f)?;
                return Ok(true);
            }

            let right_is_collection_or_call_like = matches!(
                inner_right_expr,
                Expression::ObjectExpression { .. }
                    | Expression::ArrayExpression { .. }
                    | Expression::TupleExpression { .. }
                    | Expression::Call { .. }
                    | Expression::New { .. }
                    | Expression::Instantiation { .. }
            );
            let right_is_anonymous_class_declaration = matches!(
                inner_right_expr,
                Expression::Declaration(declaration_id)
                    if matches!(
                        f.context().tree.get(*declaration_id),
                        Declaration::Class { descriptor, .. } if descriptor.name.is_none()
                    )
            );
            if (right_is_collection_or_call_like || right_is_anonymous_class_declaration)
                && !right_has_prefix_annotation
                && !right_has_between_comment
            {
                write!(
                    f,
                    [group(&format_args![
                        left,
                        space_before_operator,
                        operator,
                        space(),
                        right
                    ])]
                )?;
                return Ok(true);
            }

            if right_handles_its_own_breaking {
                let right_prefers_operator_break = if right_is_chain {
                    !right_is_lambda
                        && (right_is_chain_tail_lambda
                            || right_has_prefix_annotation
                            || right_has_between_comment)
                } else {
                    !right_is_lambda
                        && (right_is_chain_tail_lambda
                            || right_has_prefix_annotation
                            || right_has_newline
                            || right_has_existing_operator_break
                            || right_has_between_comment)
                        && (right_is_long
                            || right_has_prefix_annotation
                            || right_has_newline
                            || right_has_existing_operator_break
                            || right_has_between_comment)
                };

                let format_break_after_operator = format_with(|f| {
                    if right_has_prefix_annotation || right_has_between_comment || right_is_sequence
                    {
                        write!(
                            f,
                            [
                                left,
                                space_before_operator,
                                operator,
                                indent(&format_args![hard_line_break(), right])
                            ]
                        )
                    } else {
                        let dedented_right = dedent(&right);
                        write!(
                            f,
                            [
                                left,
                                space_before_operator,
                                operator,
                                indent(&format_args![hard_line_break(), dedented_right])
                            ]
                        )
                    }
                });

                let format_inline = format_with(|f| {
                    group(&format_args![
                        left,
                        space_before_operator,
                        operator,
                        space(),
                        right
                    ])
                    .format(f)
                });

                if right_prefers_operator_break {
                    best_fitting![format_break_after_operator, format_inline]
                        .with_mode(BestFittingMode::AllLines)
                        .format(f)?;
                } else {
                    format_inline.format(f)?;
                }
            } else {
                let left_has_newline = f.context().has_newline(f.context().get_span(*left));
                let left_inner_id = transparent_inner_expression(f.context(), *left);
                let left_is_expanded_object_target = matches!(
                    f.context().tree.get(left_inner_id),
                    Expression::ObjectExpression { properties, .. }
                        if properties.len() > 2
                            && is_assignment_left_target(f.context(), left_inner_id)
                );
                let right_is_short_object = matches!(
                    inner_right_expr,
                    Expression::ObjectExpression { properties, .. } if properties.len() <= 3
                );
                let right_is_short_atomic =
                    is_trivial_expression(f.context().tree, inner_right_expr)
                        && expression_source_len(f.context(), inner_right_id)
                            <= usize::from(f.context().options.line_width) / 3;
                if (left_has_newline || left_is_expanded_object_target)
                    && (right_is_short_object || right_is_short_atomic)
                {
                    write!(f, [left, space_before_operator, operator, space(), right])?;
                } else {
                    // other expressions get indented on break
                    write!(
                        f,
                        [group(&format_args![
                            left,
                            space_before_operator,
                            operator,
                            indent(&format_args![soft_line_break_or_space(), right])
                        ])]
                    )?;
                }
            }
        }

        // debugger
        Expression::Debugger => {
            write!(f, [token("debugger")])?;
        }

        // stub (placeholder for annotation-only files)
        Expression::Stub => {}

        // error
        Expression::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}
