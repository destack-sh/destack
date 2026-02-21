use super::argument::{
    TreeExpressionArgument, should_force_break_tree_attributes, tree_argument_is_wrapped_in_braces,
};
use super::children::{
    tree_child_breaks_element, tree_children_have_blank_line_between,
    tree_text_boundary_separator_space, tree_text_is_whitespace_only,
};
use crate::expression::{
    Argument, DestackFormatContext, DestackFormatter, Expression, FormatResult, LocalNodeId,
    NodeTree, ScalarLiteral, block_indent, empty_line, expand_parent,
    expression_has_static_type_arguments, format_with, group, hard_line_break, if_group_breaks,
    if_group_fits_on_line, indent, soft_block_indent, soft_line_break, soft_line_break_or_space,
    space, token, transparent_inner_expression,
};
use destack_ast::{IfKind, NodeType};
use destack_fir::format::{Buffer, Format};
use destack_fir::{format_args, write};

/// Store computed tree-child layout facts for one tree body.
#[derive(Clone, Copy, Debug, Default)]
struct TreeChildrenLayoutFacts {
    all_tree_children: bool,
    only_tree_or_comment_children: bool,
    force_break: bool,
    force_break_with_fill: bool,
}

/// Return the value expression id for one tree child argument.
fn tree_child_value_id(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> LocalNodeId<Expression> {
    let argument = tree.get(argument_id);
    match argument {
        Argument::Positional { value, .. }
        | Argument::Spread { value, .. }
        | Argument::Named { value, .. }
        | Argument::Labeled { value, .. } => *value,
    }
}

/// Build tree-child layout facts for one tree body.
fn collect_tree_children_layout_facts(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
    force_break_attributes: bool,
) -> TreeChildrenLayoutFacts {
    let tree = context.tree;
    let tree_child_count = elements
        .iter()
        .filter(|element_id| {
            let value_id = tree_child_value_id(tree, **element_id);
            matches!(tree.get(value_id), Expression::TreeExpression { .. })
        })
        .count();
    let expression_child_count = elements
        .iter()
        .filter(|element_id| {
            let value_id = tree_child_value_id(tree, **element_id);
            let value_id = transparent_inner_expression(context, value_id);

            !matches!(
                tree.get(value_id),
                Expression::TreeExpression { .. }
                    | Expression::Stub
                    | Expression::ScalarLiteral(ScalarLiteral::String(_))
                    | Expression::ScalarLiteral(ScalarLiteral::Character(_))
            )
        })
        .count();
    let has_breaking_child = elements
        .iter()
        .any(|element_id| tree_child_breaks_element(context, *element_id));
    let has_braced_whitespace_child = elements.iter().any(|element_id| {
        let is_whitespace_only =
            tree_text_is_whitespace_only(context, *element_id).is_some_and(|(is_only, _)| is_only);
        is_whitespace_only && tree_argument_is_wrapped_in_braces(context, *element_id)
    });
    let has_non_whitespace_text_child = elements.iter().any(|element_id| {
        tree_text_is_whitespace_only(context, *element_id).is_some_and(|(is_only, _)| !is_only)
    });
    let has_newline_whitespace_text_child = elements.iter().any(|element_id| {
        tree_text_is_whitespace_only(context, *element_id)
            .is_some_and(|(is_only, has_newline)| is_only && has_newline)
            && !tree_argument_is_wrapped_in_braces(context, *element_id)
    });
    let all_tree_children = tree_child_count == elements.len();
    let only_tree_or_comment_children = elements.iter().all(|element_id| {
        let value_id = tree_child_value_id(tree, *element_id);
        let value_id = transparent_inner_expression(context, value_id);
        matches!(
            tree.get(value_id),
            Expression::TreeExpression { .. } | Expression::Stub
        )
    });

    let has_tree_child = tree_child_count > 0;
    let has_multiple_tree_children = tree_child_count >= 2;
    let has_multiple_expression_children = expression_child_count >= 2;
    let has_tree_and_expression_children = has_tree_child && expression_child_count > 0;
    let has_tree_and_text_children = has_tree_child && has_non_whitespace_text_child;
    let force_break = if context.options.language_type.is_destack() {
        force_break_attributes
            || has_breaking_child
            || has_tree_child
            || has_multiple_expression_children
            || has_newline_whitespace_text_child
    } else {
        force_break_attributes
            || has_breaking_child
            || has_multiple_tree_children
            || has_multiple_expression_children
            || has_tree_and_expression_children
            || (has_tree_child && has_braced_whitespace_child)
            || has_newline_whitespace_text_child
    };
    let force_break_with_fill = !context.options.language_type.is_destack()
        && force_break
        && has_tree_and_text_children
        && expression_child_count == 0
        && !has_tree_and_expression_children;

    TreeChildrenLayoutFacts {
        all_tree_children,
        only_tree_or_comment_children,
        force_break,
        force_break_with_fill,
    }
}

/// Write one tree closing tag.
fn write_tree_closing_tag<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    write!(f, [token("</")])?;
    if let Some(left) = left {
        let left_expression = f.context().tree.get(*left);
        if let Expression::Path { path, .. } = left_expression {
            write!(f, [path])?;
        } else {
            write!(f, [left])?;
        }
    }
    write!(f, [token(">")])?;
    Ok(())
}

/// Format tree children in one-child-per-line mode.
fn format_tree_children_multiline<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let mut wrote_child = false;
    let mut pending_blank_line = false;
    let mut previous_emitted_argument: Option<LocalNodeId<Argument>> = None;

    for element_id in elements {
        let whitespace_info = tree_text_is_whitespace_only(f.context(), *element_id);
        let is_whitespace_only =
            whitespace_info.is_some_and(|(is_whitespace_only, _)| is_whitespace_only);
        let has_blank_line = whitespace_info.is_some_and(|(_, has_blank_line)| has_blank_line);
        let is_braced_whitespace =
            tree_argument_is_wrapped_in_braces(f.context(), *element_id) && is_whitespace_only;

        if is_whitespace_only && !is_braced_whitespace {
            if has_blank_line && wrote_child {
                pending_blank_line = true;
            }
            continue;
        }

        // keep explicit `{ " " }` style separators attached to previous child
        if is_braced_whitespace {
            if !wrote_child {
                continue;
            }
            write!(
                f,
                [TreeExpressionArgument {
                    argument_id: *element_id
                }]
            )?;
            continue;
        }

        if wrote_child {
            let has_blank_line_between_children =
                previous_emitted_argument.is_some_and(|previous_argument_id| {
                    tree_children_have_blank_line_between(
                        f.context(),
                        previous_argument_id,
                        *element_id,
                    )
                });

            if pending_blank_line || has_blank_line_between_children {
                write!(f, [empty_line()])?;
                pending_blank_line = false;
            } else {
                write!(f, [hard_line_break()])?;
            }
        }

        write!(
            f,
            [TreeExpressionArgument {
                argument_id: *element_id
            }]
        )?;
        wrote_child = true;
        previous_emitted_argument = Some(*element_id);
    }

    Ok(())
}

/// Format tree children with one tree child per line.
fn format_tree_children_tree_per_line<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    for (index, element_id) in elements.iter().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        write!(
            f,
            [TreeExpressionArgument {
                argument_id: *element_id
            }]
        )?;
    }

    Ok(())
}

/// Return whether one tree child should participate in mixed fill rendering.
fn tree_child_is_fill_visible(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let whitespace_info = tree_text_is_whitespace_only(context, argument_id);
    let is_whitespace_only =
        whitespace_info.is_some_and(|(is_whitespace_only, _)| is_whitespace_only);
    if !is_whitespace_only {
        return true;
    }

    tree_argument_is_wrapped_in_braces(context, argument_id)
}

/// Build separator decisions between adjacent fill-visible tree children.
fn collect_tree_fill_separators(
    context: &DestackFormatContext<'_>,
    inline_elements: &[LocalNodeId<Argument>],
) -> Vec<(bool, bool)> {
    let whitespace_flags = inline_elements
        .iter()
        .map(|element_id| tree_text_is_whitespace_only(context, *element_id))
        .map(|info| info.unwrap_or((false, false)))
        .collect::<Vec<_>>();
    let boundary_spaces = inline_elements
        .iter()
        .map(|element_id| {
            tree_text_boundary_separator_space(context, *element_id).unwrap_or((false, false))
        })
        .collect::<Vec<_>>();
    let inline_child_breaks = inline_elements
        .iter()
        .map(|element_id| tree_child_breaks_element(context, *element_id))
        .collect::<Vec<_>>();

    let mut separators = Vec::with_capacity(inline_elements.len());
    separators.push((false, false));
    for index in 1..inline_elements.len() {
        let prev_is_whitespace_only = whitespace_flags[index - 1].0;
        let current_is_whitespace_only = whitespace_flags[index].0;
        let force_hard_break = inline_child_breaks[index - 1] || inline_child_breaks[index];
        if prev_is_whitespace_only || current_is_whitespace_only {
            separators.push((false, force_hard_break));
            continue;
        }

        let prev_trailing_space = boundary_spaces[index - 1].1;
        let current_leading_space = boundary_spaces[index].0;
        let boundary_prefers_line_break = prev_trailing_space || current_leading_space;
        let force_hard_break = force_hard_break || boundary_prefers_line_break;
        separators.push((
            prev_trailing_space || current_leading_space,
            force_hard_break,
        ));
    }

    separators
}

/// Return whether one tree child is a multiline tree expression in source.
fn tree_child_is_multiline_tree_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = tree_child_value_id(context.tree, argument_id);
    matches!(
        context.tree.get(value_id),
        Expression::TreeExpression { .. }
    ) && context.node_has_newline(value_id)
}

/// Return whether one tree literal has braced-whitespace seams around multiline tree children.
fn tree_literal_has_multiline_whitespace_tree_seam(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    elements.iter().enumerate().any(|(index, element_id)| {
        let is_braced_whitespace = tree_text_is_whitespace_only(context, *element_id)
            .is_some_and(|(is_whitespace_only, _)| is_whitespace_only)
            && tree_argument_is_wrapped_in_braces(context, *element_id);
        if !is_braced_whitespace {
            return false;
        }

        let previous_is_multiline_tree = index
            .checked_sub(1)
            .and_then(|previous_index| elements.get(previous_index).copied())
            .is_some_and(|argument_id| {
                tree_child_is_multiline_tree_expression(context, argument_id)
            });
        let next_is_multiline_tree = elements.get(index + 1).copied().is_some_and(|argument_id| {
            tree_child_is_multiline_tree_expression(context, argument_id)
        });

        previous_is_multiline_tree || next_is_multiline_tree
    })
}

/// Format mixed tree children with fill separators.
fn format_tree_children_fill<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let inline_elements = elements
        .iter()
        .copied()
        .filter(|element_id| tree_child_is_fill_visible(f.context(), *element_id))
        .collect::<Vec<_>>();
    if inline_elements.is_empty() {
        return Ok(());
    }

    let separators = collect_tree_fill_separators(f.context(), &inline_elements);
    let braced_whitespace_flags = inline_elements
        .iter()
        .map(|element_id| {
            tree_text_is_whitespace_only(f.context(), *element_id)
                .is_some_and(|(is_whitespace_only, _)| is_whitespace_only)
                && tree_argument_is_wrapped_in_braces(f.context(), *element_id)
        })
        .collect::<Vec<_>>();
    let keep_expression_whitespace_flags = inline_elements
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if !braced_whitespace_flags[index] {
                return false;
            }

            let previous_forces_break = separators[index].1;
            let next_forces_break = separators
                .get(index + 1)
                .is_some_and(|(_, force_break)| *force_break);
            let previous_is_multiline_tree = index
                .checked_sub(1)
                .and_then(|previous_index| inline_elements.get(previous_index).copied())
                .is_some_and(|argument_id| {
                    tree_child_is_multiline_tree_expression(f.context(), argument_id)
                });
            let next_is_multiline_tree =
                inline_elements
                    .get(index + 1)
                    .copied()
                    .is_some_and(|argument_id| {
                        tree_child_is_multiline_tree_expression(f.context(), argument_id)
                    });

            previous_forces_break
                || next_forces_break
                || previous_is_multiline_tree
                || next_is_multiline_tree
        })
        .collect::<Vec<_>>();
    let tree_expression_flags = inline_elements
        .iter()
        .map(|argument_id| {
            let value_id = tree_child_value_id(f.context().tree, *argument_id);
            matches!(
                f.context().tree.get(value_id),
                Expression::TreeExpression { .. }
            )
        })
        .collect::<Vec<_>>();

    let mut fill = f.fill();
    for (index, element_id) in inline_elements.iter().enumerate() {
        let (should_insert_space_inline, force_break) = separators[index];
        let separator = format_with(|f| {
            if index == 0 {
                return Ok(());
            }

            if force_break {
                write!(f, [hard_line_break()])?;
            } else if should_insert_space_inline {
                write!(f, [soft_line_break_or_space()])?;
            } else {
                write!(f, [token("")])?;
            }

            Ok(())
        });

        let is_braced_whitespace = braced_whitespace_flags[index];
        if is_braced_whitespace {
            let keep_expression_whitespace = keep_expression_whitespace_flags[index];
            let whitespace_argument_id = *element_id;
            let whitespace_entry = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                if keep_expression_whitespace {
                    let value_id = tree_child_value_id(f.context().tree, whitespace_argument_id);
                    write!(f, [token("{"), value_id, token("}"), hard_line_break()])?;
                } else {
                    write!(f, [space()])?;
                }

                Ok(())
            });
            fill.entry(&separator, &whitespace_entry);
        } else {
            let entry = TreeExpressionArgument {
                argument_id: *element_id,
            };
            let previous_keeps_expression_whitespace =
                index.checked_sub(1).is_some_and(|previous_index| {
                    braced_whitespace_flags[previous_index]
                        && keep_expression_whitespace_flags[previous_index]
                });
            if tree_expression_flags[index] && previous_keeps_expression_whitespace {
                let expanded_entry = group(&entry).should_expand(true);
                fill.entry(&separator, &expanded_entry);
            } else {
                fill.entry(&separator, &entry);
            }
        }
    }

    fill.finish()
}

/// Format tree children using the selected layout policy.
fn format_tree_children<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
    layout_facts: TreeChildrenLayoutFacts,
) -> FormatResult<()> {
    if layout_facts.force_break && !layout_facts.force_break_with_fill && elements.len() > 1 {
        return format_tree_children_multiline(f, elements);
    }

    if (layout_facts.all_tree_children || layout_facts.only_tree_or_comment_children)
        && elements.len() > 1
    {
        return format_tree_children_tree_per_line(f, elements);
    }

    format_tree_children_fill(f, elements)
}

/// Decide whether a tree literal should break across multiple lines.
pub(crate) fn tree_literal_should_break(
    context: &DestackFormatContext<'_>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> bool {
    let force_break_attributes = arguments
        .as_ref()
        .is_some_and(|arguments| should_force_break_tree_attributes(context, arguments));

    let Some(elements) = elements else {
        return force_break_attributes;
    };

    if elements.is_empty() {
        return force_break_attributes;
    }

    let layout_facts =
        collect_tree_children_layout_facts(context, elements, force_break_attributes);
    layout_facts.force_break
}

/// Return whether a tree literal should be wrapped in parentheses when it breaks.
pub(crate) fn tree_literal_wraps_on_break(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return true;
    };

    match parent_type {
        NodeType::Expression => {
            let parent_id = LocalNodeId::<Expression>::new(parent_id);
            match context.tree.get(parent_id) {
                // explicit parentheses already control wrapping
                Expression::Parenthesized { .. } => false,
                // jsx-like containers and conditional branches keep children unwrapped
                Expression::ArrayExpression { .. }
                | Expression::TupleExpression { .. }
                | Expression::TreeExpression { .. }
                | Expression::If {
                    kind: IfKind::Ternary,
                    ..
                } => false,
                // standalone jsx statements stay unwrapped
                Expression::Statement(_) => false,
                // return handles jsx wrapping at the statement formatter level
                Expression::Return { .. } => false,
                _ => true,
            }
        }
        NodeType::Argument => {
            let Some((grand_id, grand_type)) = context.parent_by_id(parent_id) else {
                return true;
            };
            if grand_type != NodeType::Expression {
                return true;
            }

            let grand_id = LocalNodeId::<Expression>::new(grand_id);
            !matches!(
                context.tree.get(grand_id),
                Expression::Call { .. }
                    | Expression::New { .. }
                    | Expression::ArrayExpression { .. }
                    | Expression::TupleExpression { .. }
                    | Expression::TreeExpression { .. }
                    | Expression::If {
                        kind: IfKind::Ternary,
                        ..
                    }
            )
        }
        _ => true,
    }
}

/// Format a tree literal expression with optional wrap-on-break parentheses.
pub(crate) fn format_tree_literal_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
    if !tree_literal_wraps_on_break(f.context(), node_id) {
        return format_tree_literal(f, node_id, left, arguments, elements);
    }

    let should_expand = tree_literal_should_break(f.context(), arguments, elements);

    write!(
        f,
        [group(&format_with(|f| {
            write!(f, [if_group_breaks(&token("("))])?;

            let formatted_tree =
                format_with(|f| format_tree_literal(f, node_id, left, arguments, elements));
            if should_expand {
                write!(f, [block_indent(&formatted_tree)])?;
            } else {
                write!(f, [soft_block_indent(&formatted_tree)])?;
            }

            write!(f, [if_group_breaks(&token(")"))])?;
            Ok(())
        }))
        .should_expand(should_expand)]
    )
}

/// Format a tree literal.
#[inline]
pub(crate) fn format_tree_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
    let has_multiline_whitespace_tree_seam = elements.as_ref().is_some_and(|elements| {
        tree_literal_has_multiline_whitespace_tree_seam(f.context(), elements)
    });
    let should_expand_tree = tree_literal_should_break(f.context(), arguments, elements)
        || has_multiline_whitespace_tree_seam;
    let force_break_attributes = arguments
        .as_ref()
        .is_some_and(|arguments| should_force_break_tree_attributes(f.context(), arguments));

    write!(
        f,
        [group(&format_with(|f| {
            // header
            write!(
                f,
                [group(&format_with(|f| {
                    // <
                    write!(f, [token("<")])?;
                    // left
                    if let Some(left) = left {
                        write!(f, [left])?;
                    }
                    // arguments
                    if let Some(arguments) = arguments {
                        let single_attr_per_line = f.context().options.single_attribute_per_line;
                        let tag_has_static_type_arguments = left.is_some_and(|left_id| {
                            expression_has_static_type_arguments(f.context(), left_id)
                        });
                        let prefer_same_line_self_closing =
                            !f.context().options.language_type.is_destack()
                                && elements.is_none()
                                && !single_attr_per_line
                                && arguments.len() > 2
                                && !tag_has_static_type_arguments;
                        let bracket_same_line =
                            f.context().options.bracket_same_line || prefer_same_line_self_closing;

                        // separator between attributes
                        let attr_separator: &dyn Format<DestackFormatContext<'ast>> =
                            if force_break_attributes
                                || (single_attr_per_line && arguments.len() > 1)
                            {
                                &hard_line_break()
                            } else {
                                &soft_line_break_or_space()
                            };

                        // format attribute list
                        let format_attrs = format_with(|f| {
                            f.join_with(attr_separator)
                                .entries(arguments.iter().map(|argument| TreeExpressionArgument {
                                    argument_id: *argument,
                                }))
                                .finish()
                        });

                        // complex attributes should expand the element
                        if force_break_attributes {
                            write!(f, [expand_parent()])?;
                        }

                        // when bracket_same_line is true, don't add trailing line break before >
                        // when false (default), soft_block_indent adds trailing soft_line_break
                        if bracket_same_line {
                            // avoid a trailing break before `>` when bracket_same_line is enabled
                            if force_break_attributes {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        indent(&format_args![hard_line_break(), format_attrs])
                                    ]
                                )?;
                            } else {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        indent(&format_args![soft_line_break(), format_attrs])
                                    ]
                                )?;
                            }
                        } else {
                            // force expansion for complex attributes in the default layout
                            if force_break_attributes {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        group(&soft_block_indent(&format_attrs))
                                            .should_expand(true)
                                    ]
                                )?;
                            } else {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        soft_block_indent(&format_attrs)
                                    ]
                                )?;
                            }
                        }
                    }
                    // /
                    if elements.is_none() {
                        let tag_has_static_type_arguments = left.is_some_and(|left_id| {
                            expression_has_static_type_arguments(f.context(), left_id)
                        });
                        let prefer_same_line_self_closing =
                            !f.context().options.language_type.is_destack()
                                && !f.context().options.single_attribute_per_line
                                && arguments
                                    .as_ref()
                                    .is_some_and(|arguments| arguments.len() > 2)
                                && !tag_has_static_type_arguments;
                        let bracket_same_line =
                            f.context().options.bracket_same_line || prefer_same_line_self_closing;
                        let has_attributes = arguments.is_some();
                        if left.is_some() || has_attributes {
                            if has_attributes {
                                // space before /> when inline, or when bracket_same_line is true
                                if bracket_same_line {
                                    write!(f, [if_group_breaks(&space())])?;
                                }
                                write!(f, [if_group_fits_on_line(&space())])?;
                            } else {
                                write!(f, [space()])?;
                            }
                        }
                        write!(f, [token("/")])?;
                    }
                    // >
                    write!(f, [token(">")])?;
                    Ok(())
                }))]
            )?;

            // body
            if let Some(elements) = elements {
                // preserve compact empty paired tags
                if elements.is_empty() {
                    write_tree_closing_tag(f, left)?;
                    return Ok(());
                }

                let layout_facts = collect_tree_children_layout_facts(
                    f.context(),
                    elements,
                    force_break_attributes,
                );
                let format_children =
                    format_with(|f| format_tree_children(f, elements, layout_facts));
                if layout_facts.force_break {
                    write!(f, [block_indent(&group(&format_children))])?;
                } else {
                    // use soft indent: stays on one line if it fits
                    write!(f, [group(&soft_block_indent(&format_children))])?;
                }

                // closing tag uses path only, no static arguments
                write_tree_closing_tag(f, left)?;
            }

            Ok(())
        }))
        .should_expand(should_expand_tree)]
    )
}
