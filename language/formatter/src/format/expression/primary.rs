use super::*;
use crate::analysis::timing::tags;
use crate::collection::{collection_nodes_have_annotations, collection_range_is_inline};
use destack_fir::{format_args, write};

// parenthesized assignment target thresholds
const PAREN_ASSIGNMENT_OBJECT_EXPAND_MIN_PROPERTIES: usize = 3;
const PAREN_ASSIGNMENT_ARRAY_EXPAND_MIN_ELEMENTS: usize = 4;

/// Return whether an expression or its wrapped declaration has prefix annotations.
fn expression_has_effective_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_prefix_annotation(expression_id) {
        return true;
    }

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => context.has_prefix_annotation(*declaration_id),
        _ => false,
    }
}

/// Return whether an expression or wrapped declaration has decorator prefix annotations.
fn expression_has_effective_decorator_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_has_decorator = context.visit_annotations(expression_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id),
                Annotation::Decorator {
                    position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                    ..
                }
            )
        })
    });
    if expression_has_decorator.unwrap_or(false) {
        return true;
    }

    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    context
        .visit_annotations(*declaration_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Decorator {
                        position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false)
}

/// Format primary expression variants.
pub(super) fn format_primary_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    let tree = f.context().tree;

    match expression {
        // path
        Expression::Path {
            path,
            static_arguments,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PATH);
            write!(f, [path])?;

            // static arguments
            if let Some(static_arguments) = static_arguments
                && !static_arguments.is_empty()
            {
                let is_single_simple = static_arguments.len() == 1
                    && is_simple_static_argument(f.context(), static_arguments[0]);
                if is_single_simple {
                    write!(f, [token("<"), static_arguments[0], token(">"),])?;
                } else {
                    format_static_argument_list(f, static_arguments)?;
                }
            }
        }

        // private identifier
        Expression::PrivateIdentifier { name } => {
            write!(f, [token("#"), *name])?;
        }

        // this
        Expression::This => {
            write!(f, [Keyword::This])?;
        }

        // super
        Expression::Super => {
            write!(f, [Keyword::Super])?;
        }

        // scalar literal
        Expression::ScalarLiteral(node) => {
            format_scalar_literal(node, tree.get_span(node_id), f)?;
        }

        // template literal
        Expression::TemplateExpression { value } => {
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // tagged template literal
        Expression::TaggedTemplateExpression { tag, value } => {
            write!(f, [tag])?;
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // type template literal
        Expression::TypeTemplateLiteral { strings, spans } => {
            format_type_template_literal(strings, spans, f)?;
        }

        // type literal
        Expression::TypeLiteral(node) => node.format(f)?,

        // type import
        Expression::TypeImport {
            target: _,
            arguments,
            qualifier,
            static_arguments,
        } => {
            write!(f, [Keyword::Import, list_like("(", ")", ",", arguments)])?;
            if let Some(qualifier) = qualifier {
                write!(f, [token("."), qualifier])?;
            }
            if let Some(static_arguments) = static_arguments {
                format_static_argument_list(f, static_arguments)?;
            }
        }

        // type infer
        Expression::TypeInfer { name, constraint } => {
            write!(f, [Keyword::Infer, space(), *name])?;
            if let Some(constraint) = constraint {
                write!(f, [space(), Keyword::Extends, space(), *constraint])?;
            }
        }

        // type predicate
        Expression::TypePredicate {
            asserts,
            subject,
            target,
        } => {
            if *asserts {
                write!(f, [Keyword::Asserts, space()])?;
            }
            match subject {
                TypePredicateSubject::Identifier(name) => {
                    write!(f, [*name])?;
                }
                TypePredicateSubject::This => {
                    write!(f, [Keyword::This])?;
                }
            }
            if let Some(target) = target {
                write!(f, [space(), Keyword::Is, space(), *target])?;
            }
        }

        // type conditional
        Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_TYPE_CONDITIONAL);
            let conditional_tail = format_with(|f| {
                write!(
                    f,
                    [
                        soft_line_break_or_space(),
                        token("?"),
                        space(),
                        then_type,
                        soft_line_break_or_space(),
                        token(":"),
                        space(),
                        else_type
                    ]
                )
            });
            let should_double_indent_tail =
                if expression_is_in_template_literal_interpolation(f.context(), node_id) {
                    f.context()
                        .expression_has_type_conditional_ancestor(node_id)
                } else {
                    false
                };
            if should_double_indent_tail {
                write!(
                    f,
                    [group(&format_args![
                        left,
                        space(),
                        Keyword::Extends,
                        space(),
                        right,
                        indent(&indent(&conditional_tail))
                    ])]
                )?;
            } else {
                write!(
                    f,
                    [group(&format_args![
                        left,
                        space(),
                        Keyword::Extends,
                        space(),
                        right,
                        indent(&conditional_tail)
                    ])]
                )?;
            }
        }

        // type mapped
        Expression::TypeMapped {
            parameter,
            modifiers,
            value,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_TYPE_MAPPED);
            let include_space = f.context().options.bracket_spacing;
            let break_parameter_clause = f.context().has_annotation(parameter.constraint)
                || f.context().node_has_newline(parameter.constraint)
                || is_expression_breakable(tree, tree.get(parameter.constraint));
            let inline_separator = if include_space {
                soft_line_break_or_space()
            } else {
                soft_line_break()
            };
            let field_terminator = if f.context().options.language_type.is_typescript() {
                ";"
            } else {
                ","
            };

            let format_parameter_clause = |f: &mut DestackFormatter<'ast, '_>,
                                           break_between_name_and_in: bool|
             -> FormatResult<()> {
                write!(f, [token("["), parameter.name])?;

                if break_between_name_and_in {
                    write!(
                        f,
                        [indent(&format_args![
                            hard_line_break(),
                            Keyword::In,
                            space(),
                            parameter.constraint
                        ])]
                    )?;
                } else {
                    write!(f, [space(), Keyword::In, space(), parameter.constraint])?;
                }

                if let Some(key_remap) = parameter.key_remap {
                    write!(f, [space(), Keyword::As, space(), key_remap])?;
                }

                write!(f, [token("]")])
            };

            let inner_multiline = format_with(|f| {
                match modifiers.readonly {
                    TypeModifier::Add => {
                        write!(f, [token("readonly"), space()])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-readonly"), space()])?;
                    }
                    TypeModifier::None => {}
                }

                format_parameter_clause(f, break_parameter_clause)?;

                match modifiers.optional {
                    TypeModifier::Add => {
                        write!(f, [token("?")])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-?")])?;
                    }
                    TypeModifier::None => {}
                }

                write!(f, [token(":"), space()])?;

                let has_value_postfix_annotations = f.context().has_postfix_annotation(*value);
                if f.context().options.language_type.is_typescript()
                    && has_value_postfix_annotations
                {
                    let value_expression = f.context().tree.get(*value);
                    write!(f, [f.context().any_prefix_annotations(*value)])?;
                    format_expression(
                        f,
                        *value,
                        value_expression,
                        directive_for_node(f.context(), *value),
                    )?;
                    write!(f, [token(field_terminator)])?;
                    write!(f, [f.context().any_infix_or_postfix_annotations(*value)])
                } else {
                    write!(f, [*value, token(field_terminator)])
                }
            });

            let inner_flat = format_with(|f| {
                match modifiers.readonly {
                    TypeModifier::Add => {
                        write!(f, [token("readonly"), space()])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-readonly"), space()])?;
                    }
                    TypeModifier::None => {}
                }

                format_parameter_clause(f, false)?;

                match modifiers.optional {
                    TypeModifier::Add => {
                        write!(f, [token("?")])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-?")])?;
                    }
                    TypeModifier::None => {}
                }

                write!(f, [token(":"), space(), *value])
            });

            let mapped_multiline = format_with(|f| {
                write!(
                    f,
                    [
                        token("{"),
                        hard_line_break(),
                        block_indent(&inner_multiline),
                        hard_line_break(),
                        token("}")
                    ]
                )
            });

            let mapped_flat = format_with(|f| {
                write!(
                    f,
                    [
                        token("{"),
                        indent(&format_args![inline_separator, inner_flat]),
                        inline_separator,
                        token("}")
                    ]
                )
            });

            let force_multiline = should_force_multiline_mapped_type(f.context(), node_id, *value);
            if force_multiline {
                write!(f, [mapped_multiline])?;
            } else {
                write!(f, [group(&mapped_flat)])?;
            }
        }

        // type index
        Expression::TypeIndex { left, index } => {
            format_type_index_expression(f, *left, *index)?;
        }

        // array literal
        Expression::ArrayExpression {
            elements: elements_ids,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_ARRAY);
            // try hugged format for single object/array elements
            if !format_hugged(f, elements_ids, HugOptions::ARRAY, None, false)? {
                if array_has_sparse_holes(f.context(), elements_ids) {
                    let span = f.context().span(node_id);
                    let has_newline_in_source = f.context().has_newline(span);
                    let has_sparse_annotations = f.context().has_infix_annotation(node_id)
                        || collection_nodes_have_annotations(f.context(), elements_ids);

                    if has_newline_in_source || has_sparse_annotations {
                        write!(
                            f,
                            [list_like("[", "]", ",", elements_ids)
                                .as_collection()
                                .should_expand(has_newline_in_source)]
                        )?;
                    } else {
                        format_sparse_array_literal(f, elements_ids)?;
                    }

                    return Ok(true);
                }

                let span = f.context().span(node_id);
                let has_newline_in_source = f.context().has_newline(span);
                let has_annotations = f.context().has_infix_annotation(node_id)
                    || collection_nodes_have_annotations(f.context(), elements_ids);

                let mut elements_are_inline_in_source = true;
                if has_annotations || (has_newline_in_source && elements_ids.len() > 1) {
                    elements_are_inline_in_source =
                        collection_range_is_inline(f.context(), elements_ids);
                }

                let mut should_expand_for_annotations = false;
                let mut can_keep_inline_boundary_comment_array = false;

                // annotation-sensitive expansion checks
                if has_annotations {
                    let has_line_comment_annotations =
                        elements_ids.iter().copied().any(|element_id| {
                            let annotation_profile =
                                f.context().ensure_argument_annotation_facts(element_id);
                            annotation_profile.has_line_comment
                        });

                    can_keep_inline_boundary_comment_array = elements_are_inline_in_source
                        && array_elements_are_fill_candidates(f.context().tree, elements_ids)
                        && array_has_only_boundary_comments(f.context(), span, elements_ids);
                    should_expand_for_annotations =
                        has_line_comment_annotations || !elements_are_inline_in_source;
                }

                let has_complex_elements = elements_ids.len() > 1
                    && elements_ids
                        .iter()
                        .copied()
                        .any(|element_id| is_complex_argument(tree, tree.get(element_id)));
                let has_single_non_trivial_multiline_element = elements_ids.len() == 1
                    && has_newline_in_source
                    && !is_trivial_argument(tree, tree.get(elements_ids[0]));
                let has_multiline_non_inline_multi_element = has_newline_in_source
                    && elements_ids.len() > 1
                    && !elements_are_inline_in_source;

                let should_expand = (should_expand_for_annotations
                    && !can_keep_inline_boundary_comment_array)
                    || has_complex_elements
                    || has_single_non_trivial_multiline_element
                    || has_multiline_non_inline_multi_element;

                if can_keep_inline_boundary_comment_array {
                    format_boundary_comment_array(f, elements_ids)?;
                } else {
                    write!(
                        f,
                        [list_like("[", "]", ",", elements_ids)
                            .as_collection()
                            .should_expand(should_expand)]
                    )?;
                }
            }
        }

        // tuple literal
        Expression::TupleExpression {
            elements: elements_ids,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_TUPLE);
            if elements_ids.is_empty() {
                write!(f, [token("()")])?;
            } else if !format_hugged(f, elements_ids, HugOptions::TUPLE, None, false)? {
                // not a single huggable element: use regular formatting
                let span = f.context().span(node_id);

                // check for annotations that require expansion
                let has_annotations = f.context().has_infix_annotation(node_id)
                    || collection_nodes_have_annotations(f.context(), elements_ids);
                let has_complex_elements = elements_ids.len() > 1
                    && elements_ids
                        .iter()
                        .copied()
                        .any(|element_id| is_complex_argument(tree, tree.get(element_id)));
                let should_expand = has_annotations
                    || has_complex_elements
                    || (f.context().has_newline(span) && elements_ids.len() > 1);
                // trailing comma disambiguates tuples from parenthesized expressions
                write!(
                    f,
                    [list_like("(", ")", ",", elements_ids)
                        .as_collection()
                        .force_trailing_separator()
                        .should_expand(should_expand)]
                )?;
            }
        }

        // sequence expression (JS/TS comma operator)
        Expression::SequenceExpression { expressions } => {
            if expressions.is_empty() {
                write!(f, [token("()")])?;
            } else {
                let format_sequence = format_with(|f| {
                    let joiner_separator = format_with(|f| {
                        write!(
                            f,
                            [
                                token(","),
                                line_postfix_boundary(),
                                soft_line_break_or_space()
                            ]
                        )
                    });
                    let mut joiner = f.join_with(joiner_separator);
                    joiner.entries(expressions);
                    joiner.finish()
                });
                if sequence_expression_needs_parens(f.context(), node_id) {
                    write!(
                        f,
                        [group(&format_args![
                            token("("),
                            format_sequence,
                            token(")")
                        ])]
                    )?;
                } else {
                    write!(f, [group(&format_sequence)])?;
                }
            }
        }

        // struct literal
        Expression::ObjectExpression { ty, properties } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_OBJECT);
            format_struct_literal(f, node_id, ty, properties)?;
        }

        // tree literal
        Expression::TreeExpression {
            left,
            arguments,
            elements,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_TREE);
            format_tree_literal_expression(f, node_id, left, arguments, elements)?;
        }

        // parenthesized
        Expression::Parenthesized { expression } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES);
            let inner_expression = tree.get(*expression);
            let should_drop_parentheses = parenthesized_should_drop(
                f.context(),
                node_id,
                *expression,
                ParenthesizedDropPolicy::ExpressionWrapper,
            );

            if should_drop_parentheses {
                write!(f, [*expression])?;
            } else {
                let has_parenthesized_leading_inner_trivia =
                    parenthesized_has_leading_inner_trivia(f.context(), node_id, *expression);
                let has_parenthesized_leading_inner_newline =
                    parenthesized_has_leading_inner_newline(f.context(), node_id, *expression);
                let has_parenthesized_leading_inner_comments =
                    parenthesized_has_leading_inner_comments(f.context(), node_id, *expression);
                let has_parenthesized_prefix_annotation =
                    expression_has_effective_prefix_annotation(f.context(), node_id)
                        || expression_has_effective_prefix_annotation(f.context(), *expression);
                let should_expand_assignment_target = match inner_expression {
                    // prefer expanded destructuring targets once they become moderately wide
                    Expression::ObjectExpression { properties, .. } => {
                        properties.len() >= PAREN_ASSIGNMENT_OBJECT_EXPAND_MIN_PROPERTIES
                            && is_assignment_left_target(f.context(), *expression)
                    }
                    Expression::ArrayExpression { elements } => {
                        elements.len() >= PAREN_ASSIGNMENT_ARRAY_EXPAND_MIN_ELEMENTS
                            && is_assignment_left_target(f.context(), *expression)
                    }
                    _ => false,
                };

                if should_expand_assignment_target {
                    write!(
                        f,
                        [group(&format_args![
                            token("("),
                            group(expression).should_expand(true),
                            token(")")
                        ])
                        .should_expand(true)]
                    )?;
                } else if let Expression::TreeExpression {
                    arguments,
                    elements,
                    ..
                } = inner_expression
                {
                    let tree_should_break =
                        tree_literal_should_break(f.context(), arguments, elements)
                            || f.context().node_has_newline(*expression);
                    if is_call_like_argument(f.context(), node_id) {
                        write!(f, [*expression])?;
                    } else if has_parenthesized_leading_inner_trivia || tree_should_break {
                        write!(
                            f,
                            [
                                token("("),
                                block_indent(&group(expression).should_expand(true)),
                                hard_line_break(),
                                token(")")
                            ]
                        )?;
                    } else {
                        write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
                    }
                } else if should_hoist_parenthesized_inner_cast_prefix_comments(
                    f.context(),
                    node_id,
                    *expression,
                ) {
                    let inner_directive = directive_for_node(f.context(), *expression);
                    let format_inner_without_prefix = format_with(|f| {
                        format_expression(
                            f,
                            *expression,
                            f.context().tree.get(*expression),
                            inner_directive,
                        )?;
                        if !matches!(
                            inner_directive,
                            Some(FormatterDirective {
                                kind: FormatterDirectiveKind::IgnoreFormat,
                                position: FormatterDirectivePosition::Postfix { .. },
                            })
                        ) {
                            write!(
                                f,
                                [f.context().any_infix_or_postfix_annotations(*expression)]
                            )?;
                        }
                        Ok(())
                    });
                    write!(f, [f.context().any_prefix_annotations(*expression)])?;
                    write!(
                        f,
                        [group(&format_args![
                            token("("),
                            format_inner_without_prefix,
                            token(")")
                        ])]
                    )?;
                } else if has_parenthesized_prefix_annotation {
                    let inner_has_decorator_prefix_annotation =
                        expression_has_effective_decorator_prefix_annotation(
                            f.context(),
                            *expression,
                        );
                    if f.context().node_has_newline(*expression)
                        || has_parenthesized_leading_inner_newline
                        || inner_has_decorator_prefix_annotation
                    {
                        write!(
                            f,
                            [
                                token("("),
                                block_indent(&group(expression).should_expand(true)),
                                hard_line_break(),
                                token(")")
                            ]
                        )?;
                    } else {
                        let line_width = usize::from(f.context().options.line_width);
                        let inline_width = expression_source_len(f.context(), *expression) + 2;
                        if inline_width <= line_width {
                            write!(f, [token("("), expression, token(")")])?;
                        } else {
                            write!(
                                f,
                                [
                                    token("("),
                                    block_indent(&group(expression).should_expand(true)),
                                    hard_line_break(),
                                    token(")")
                                ]
                            )?;
                        }
                    }
                } else if matches!(inner_expression, Expression::TypeConditional { .. }) {
                    write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
                } else if matches!(
                    inner_expression,
                    Expression::Binary {
                        operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                        ..
                    }
                ) && f.context().node_has_newline(*expression)
                {
                    write!(
                        f,
                        [
                            token("("),
                            block_indent(&group(expression).should_expand(true)),
                            hard_line_break(),
                            token(")")
                        ]
                    )?;
                } else if has_parenthesized_leading_inner_trivia {
                    if has_parenthesized_leading_inner_newline {
                        let line_width = usize::from(f.context().options.line_width);
                        let available_width = assignment_like_remaining_width(f.context(), node_id)
                            .unwrap_or(line_width);
                        let inline_width = expression_source_len(f.context(), *expression) + 2;
                        let should_keep_multiline = has_parenthesized_leading_inner_comments
                            || inline_width > available_width;
                        if !should_keep_multiline {
                            write!(f, [token("("), expression, token(")")])?;
                            let boundary_comments = collect_parenthesized_boundary_comments(
                                f.context(),
                                node_id,
                                *expression,
                            );
                            for comment in boundary_comments {
                                write!(f, [space(), text(comment.as_str())])?;
                            }
                            return Ok(true);
                        }

                        write!(
                            f,
                            [
                                token("("),
                                block_indent(&expression),
                                hard_line_break(),
                                token(")")
                            ]
                        )?;
                    } else {
                        write!(f, [token("("), expression, token(")")])?;
                    }
                } else {
                    let line_width = usize::from(f.context().options.line_width);
                    let available_width =
                        assignment_like_remaining_width(f.context(), node_id).unwrap_or(line_width);
                    let inline_width = expression_source_len(f.context(), *expression) + 2;
                    let is_await_wrapped_chain = matches!(
                        tree.get(*expression),
                        Expression::Await { expression }
                            | Expression::AwaitMaybe { expression }
                            if is_expression_chain(tree, *expression) || is_chain_root(tree, *expression)
                    );
                    let should_expand_parenthesized_chain =
                        (is_expression_chain(tree, *expression) || is_await_wrapped_chain)
                            && inline_width > available_width;

                    if should_expand_parenthesized_chain {
                        write!(
                            f,
                            [
                                token("("),
                                block_indent(&group(expression).should_expand(true)),
                                hard_line_break(),
                                token(")")
                            ]
                        )?;
                    } else {
                        write!(f, [token("("), expression, token(")")])?;
                    }
                }

                let boundary_comments =
                    collect_parenthesized_boundary_comments(f.context(), node_id, *expression);
                for comment in boundary_comments {
                    write!(f, [space(), text(comment.as_str())])?;
                }
            }
        }
        _ => return Ok(false),
    }

    Ok(true)
}

/// Return whether an array literal contains sparse elision slots.
fn array_has_sparse_holes(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    elements
        .iter()
        .copied()
        .any(|argument_id| argument_is_sparse_hole(context, argument_id))
}

/// Return whether an array element argument is a sparse hole.
fn argument_is_sparse_hole(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Argument::Positional { value, .. } = context.tree.get(argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, *value);
    matches!(context.tree.get(value_id), Expression::Stub)
}

/// Format sparse arrays while preserving elision comma count.
fn format_sparse_array_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [token("[")])?;

    let mut expects_element = true;
    for (index, argument_id) in elements.iter().copied().enumerate() {
        let is_hole = argument_is_sparse_hole(f.context(), argument_id);

        if !expects_element {
            write!(f, [token(",")])?;
            if !is_hole {
                write!(f, [space()])?;
            }
            expects_element = true;
        }

        if is_hole {
            write!(f, [token(",")])?;
            let next_is_non_hole = elements
                .get(index + 1)
                .is_some_and(|next_id| !argument_is_sparse_hole(f.context(), *next_id));
            if next_is_non_hole {
                write!(f, [space()])?;
            }
            continue;
        }

        write!(f, [argument_id])?;
        expects_element = false;
    }

    write!(f, [token("]")])?;
    Ok(())
}
