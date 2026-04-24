use crate::format::annotation::{
    FormatTrailingComments, block_infix_annotations, format_leading_comments,
};
use crate::format::chain::transparent_inner_expression;
use crate::format::declaration::expression_is_in_statement_position;
use crate::format::expression::format_generic_argument_list;
use crate::format::tree::{
    is_jsx_whitespace_char, should_force_break_tree_attributes, tree_argument_is_wrapped_in_braces,
    tree_child_breaks_element, tree_children_have_blank_line_between, tree_text_is_whitespace_only,
    write_tree_expression_argument,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Declaration, Expression, FunctionKind, GenericArgument, IfKind, LocalNodeId,
    NodeTree, NodeType, ScalarLiteral,
};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{
    block_indent, empty_line, expand_parent, format_with, group, hard_line_break, if_group_breaks,
    if_group_fits_on_line, indent, soft_block_indent, soft_line_break, soft_line_break_or_space,
    space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::QuoteStyle;

/// Return the value expression id for one tree child argument.
fn tree_child_value_id(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    let argument = tree.get(argument_id);
    match argument {
        Argument::Positional { value, .. }
        | Argument::Spread { value, .. }
        | Argument::Named { value, .. }
        | Argument::Labeled { value, .. } => Some(*value),
        Argument::Error => None,
    }
}

/// Build tree-child layout data for one tree body.
fn tree_children_layout(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
    force_break_attributes: bool,
) -> (bool, bool, bool, bool) {
    let tree = context.tree;
    let mut tree_child_count = 0usize;
    let mut expression_child_count = 0usize;
    let mut has_breaking_child = false;
    let mut has_non_whitespace_text_child = false;
    let mut has_newline_whitespace_text_child = false;
    let mut only_tree_or_comment_children = true;

    for element_id in elements {
        let value_id = tree_child_value_id(tree, *element_id);
        let Some(value_id) = value_id else {
            expression_child_count += 1;
            only_tree_or_comment_children = false;
            continue;
        };
        let value_id = transparent_inner_expression(context, value_id);
        let value = tree.get(value_id);

        if matches!(value, Expression::TreeExpression { .. }) {
            tree_child_count += 1;
        }

        if !matches!(
            value,
            Expression::TreeExpression { .. }
                | Expression::Stub
                | Expression::ScalarLiteral(ScalarLiteral::String(_))
                | Expression::ScalarLiteral(ScalarLiteral::Character(_))
        ) {
            expression_child_count += 1;
        }

        if tree_child_breaks_element(context, *element_id) {
            has_breaking_child = true;
        }

        let whitespace_info = tree_text_is_whitespace_only(context, *element_id);
        if let Some((is_whitespace_only, has_newline)) = whitespace_info {
            if !is_whitespace_only {
                has_non_whitespace_text_child = true;
            }

            if is_whitespace_only
                && has_newline
                && !tree_argument_is_wrapped_in_braces(context, *element_id)
            {
                has_newline_whitespace_text_child = true;
            }
        }

        // tree per line layout ignores pure whitespace text separators
        let is_non_content_text_separator =
            whitespace_info.is_some_and(|(is_whitespace_only, _)| is_whitespace_only);
        if !matches!(value, Expression::TreeExpression { .. } | Expression::Stub)
            && !is_non_content_text_separator
        {
            only_tree_or_comment_children = false;
        }
    }

    let has_tree_child = tree_child_count > 0;
    let has_multiple_expression_children = expression_child_count >= 2;
    let has_tree_and_expression_children = has_tree_child && expression_child_count > 0;
    let has_tree_and_text_children = has_tree_child && has_non_whitespace_text_child;
    let tree_children_force_break = context.options.language_type.is_destack();
    let force_break =
        // tree child forms break as soon as any structured child appears
        (tree_children_force_break
            && (force_break_attributes
                || has_breaking_child
                || has_tree_child
                || has_multiple_expression_children
                || has_newline_whitespace_text_child))
            // tag forms keep text only content inline a bit longer
            || (!tree_children_force_break
                && (force_break_attributes
                    || has_breaking_child
                    || has_tree_child
                    || has_multiple_expression_children));
    let force_break_with_fill = !context.options.language_type.is_destack()
        && force_break
        && has_tree_and_text_children
        && expression_child_count == 0
        && !has_tree_and_expression_children;

    (
        tree_child_count == elements.len(),
        only_tree_or_comment_children,
        force_break,
        force_break_with_fill,
    )
}

/// Write one tree closing tag.
fn write_tree_closing_tag<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    write!(f, [token("</")])?;
    if let Some(left) = left {
        let left_expression = f.context().tree.get(*left);
        if let Expression::QualifiedReference { path, .. } = left_expression {
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

        if is_whitespace_only {
            if has_blank_line && wrote_child {
                pending_blank_line = true;
            }
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
            [format_with(|f| write_tree_expression_argument(
                f,
                *element_id,
                None,
            ))]
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
            [format_with(|f| write_tree_expression_argument(
                f,
                *element_id,
                None,
            ))]
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
    let Some((is_whitespace_only, _)) = whitespace_info else {
        return true;
    };

    if !is_whitespace_only {
        return true;
    }

    false
}

/// Build separator rules between adjacent fill-visible tree children.
fn tree_fill_separators(
    context: &DestackFormatContext<'_>,
    all_elements: &[LocalNodeId<Argument>],
    inline_elements: &[LocalNodeId<Argument>],
) -> Vec<(bool, bool)> {
    // map each visible element back to its position in the original child list
    let mut inline_indices = Vec::with_capacity(inline_elements.len());
    let mut all_index = 0usize;
    for inline_element_id in inline_elements {
        while all_index < all_elements.len() && all_elements[all_index] != *inline_element_id {
            all_index += 1;
        }
        if all_index >= all_elements.len() {
            return vec![(false, false); inline_elements.len()];
        }

        inline_indices.push(all_index);
        all_index += 1;
    }

    let inline_child_breaks = inline_elements
        .iter()
        .map(|element_id| tree_child_breaks_element(context, *element_id))
        .collect::<Vec<_>>();

    let mut separators = Vec::with_capacity(inline_elements.len());
    separators.push((false, false));
    for index in 1..inline_elements.len() {
        let force_hard_break = inline_child_breaks[index - 1] || inline_child_breaks[index];

        // preserve dropped jsx whitespace children between visible entries
        let previous_all_index = inline_indices[index - 1];
        let current_all_index = inline_indices[index];
        let skipped_elements = &all_elements[previous_all_index + 1..current_all_index];
        let skipped_has_newline_whitespace = skipped_elements.iter().any(|element_id| {
            tree_text_is_whitespace_only(context, *element_id)
                .is_some_and(|(is_whitespace_only, has_newline)| is_whitespace_only && has_newline)
        });
        let skipped_has_inline_whitespace = skipped_elements.iter().any(|element_id| {
            tree_text_is_whitespace_only(context, *element_id)
                .is_some_and(|(is_whitespace_only, has_newline)| is_whitespace_only && !has_newline)
        });
        if skipped_has_newline_whitespace {
            separators.push((true, false));
            continue;
        }

        if skipped_has_inline_whitespace {
            if force_hard_break {
                separators.push((true, false));
            } else {
                separators.push((false, true));
            }
            continue;
        }

        let previous_spacing_flags =
            tree_text_adjacent_spacing_flags(context, inline_elements[index - 1]);
        let current_spacing_flags =
            tree_text_adjacent_spacing_flags(context, inline_elements[index]);
        let previous_trailing_has_newline = previous_spacing_flags
            .is_some_and(|(_, trailing_has_newline, _, _)| trailing_has_newline);
        let current_leading_has_newline =
            current_spacing_flags.is_some_and(|(leading_has_newline, _, _, _)| leading_has_newline);
        let previous_trailing_has_inline_whitespace =
            previous_spacing_flags.is_some_and(|(_, _, _, trailing_has_inline_whitespace)| {
                trailing_has_inline_whitespace
            });
        let current_leading_has_inline_whitespace = current_spacing_flags
            .is_some_and(|(_, _, leading_has_inline_whitespace, _)| leading_has_inline_whitespace);
        let force_hard_break =
            force_hard_break || previous_trailing_has_newline || current_leading_has_newline;
        if force_hard_break {
            separators.push((true, false));
        } else if previous_trailing_has_inline_whitespace || current_leading_has_inline_whitespace {
            separators.push((false, true));
        } else {
            separators.push((false, false));
        }
    }

    separators
}

/// Return one tree-text adjacent spacing state.
fn tree_text_adjacent_spacing_flags(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool, bool, bool)> {
    let Argument::Positional { value, .. } = context.tree.get(argument_id) else {
        return None;
    };
    let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = context.tree.get(*value)
    else {
        return None;
    };

    let text = context.strings.get(*string_id);
    let leading_end = text
        .char_indices()
        .find(|(_, c)| !is_jsx_whitespace_char(*c))
        .map_or(text.len(), |(index, _)| index);
    let trailing_start = text
        .char_indices()
        .rev()
        .find(|(_, c)| !is_jsx_whitespace_char(*c))
        .map_or(0, |(index, c)| index + c.len_utf8());

    let leading_whitespace = &text[..leading_end];
    let trailing_whitespace = &text[trailing_start..];
    let leading_has_newline = leading_whitespace.contains(['\n', '\r']);
    let trailing_has_newline = trailing_whitespace.contains(['\n', '\r']);

    Some((
        leading_has_newline,
        trailing_has_newline,
        !leading_whitespace.is_empty() && !leading_has_newline,
        !trailing_whitespace.is_empty() && !trailing_has_newline,
    ))
}

/// Return whether one whitespace-only child run contains inline or newline spacing.
fn tree_whitespace_run_spacing(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
) -> (bool, bool) {
    let mut has_inline_whitespace = false;
    let mut has_newline_whitespace = false;

    for element_id in elements {
        let Some((is_whitespace_only, has_newline)) =
            tree_text_is_whitespace_only(context, *element_id)
        else {
            continue;
        };
        if !is_whitespace_only {
            continue;
        }

        if has_newline {
            has_newline_whitespace = true;
        } else {
            has_inline_whitespace = true;
        }
    }

    if has_newline_whitespace {
        return (true, false);
    }

    if has_inline_whitespace {
        return (false, true);
    }

    (false, false)
}

/// Return one JSX space token that matches the configured quote style.
fn tree_jsx_space_token(context: &DestackFormatContext<'_>) -> &'static str {
    match context.options.quote_style {
        QuoteStyle::Single => "{' '}",
        QuoteStyle::Double | QuoteStyle::Semantic => "{\" \"}",
    }
}

/// Emit one JSX whitespace separator that stays inline in flat mode.
fn write_tree_jsx_whitespace_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    let jsx_space = token(tree_jsx_space_token(f.context()));
    write!(
        f,
        [
            if_group_breaks(&format_args![jsx_space, soft_line_break()]),
            if_group_fits_on_line(&space())
        ]
    )
}

/// Emit one JSX whitespace token (`{" "}` or `{' '}`) unconditionally.
fn write_tree_jsx_whitespace_token<'ast>(f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
    write!(f, [token(tree_jsx_space_token(f.context()))])
}

/// Return whether one tree child is a multiline tree expression in source.
fn tree_child_is_multiline_tree_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = tree_child_value_id(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);
    matches!(
        context.tree.get(value_id),
        Expression::TreeExpression { .. }
    ) && context.node_has_newline(value_id)
}

/// Return whether one tree literal has braced-whitespace separators around multiline tree children.
fn tree_literal_has_multiline_whitespace_separator(
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
        let (has_newline_spacing, has_inline_spacing) =
            tree_whitespace_run_spacing(f.context(), elements);
        if has_newline_spacing {
            write!(f, [hard_line_break()])?;
        } else if has_inline_spacing {
            write_tree_jsx_whitespace_token(f)?;
        }

        return Ok(());
    }

    let first_inline_element_id = inline_elements.first().copied();
    let last_inline_element_id = inline_elements.last().copied();
    let first_inline_index = first_inline_element_id
        .and_then(|element_id| {
            elements
                .iter()
                .position(|candidate| *candidate == element_id)
        })
        .unwrap_or(0);
    let last_inline_index = last_inline_element_id
        .and_then(|element_id| {
            elements
                .iter()
                .rposition(|candidate| *candidate == element_id)
        })
        .unwrap_or(elements.len().saturating_sub(1));

    let prefix_spacing = tree_whitespace_run_spacing(f.context(), &elements[..first_inline_index]);
    let suffix_spacing =
        tree_whitespace_run_spacing(f.context(), &elements[last_inline_index + 1..]);
    let prefix_spacing_flags = first_inline_element_id
        .and_then(|element_id| tree_text_adjacent_spacing_flags(f.context(), element_id));
    let suffix_spacing_flags = last_inline_element_id
        .and_then(|element_id| tree_text_adjacent_spacing_flags(f.context(), element_id));
    let prefix_has_adjacent_newline =
        prefix_spacing_flags.is_some_and(|(leading_has_newline, _, _, _)| leading_has_newline);
    let suffix_has_adjacent_newline =
        suffix_spacing_flags.is_some_and(|(_, trailing_has_newline, _, _)| trailing_has_newline);
    let prefix_has_adjacent_inline_whitespace = prefix_spacing_flags
        .is_some_and(|(_, _, leading_has_inline_whitespace, _)| leading_has_inline_whitespace);
    let suffix_has_adjacent_inline_whitespace = suffix_spacing_flags
        .is_some_and(|(_, _, _, trailing_has_inline_whitespace)| trailing_has_inline_whitespace);

    let separators = tree_fill_separators(f.context(), elements, &inline_elements);

    if prefix_spacing.0 {
        write!(f, [hard_line_break()])?;
    } else if prefix_spacing.1 {
        write_tree_jsx_whitespace_separator(f)?;
    } else if prefix_has_adjacent_newline {
        write!(f, [hard_line_break()])?;
    } else if prefix_has_adjacent_inline_whitespace {
        write_tree_jsx_whitespace_separator(f)?;
    }

    let mut fill = f.fill();
    for (index, element_id) in inline_elements.iter().enumerate() {
        let (separator_is_hard_break, separator_has_inline_whitespace) = separators[index];
        let separator = format_with(|f| {
            if index == 0 {
                return Ok(());
            }

            if separator_is_hard_break {
                write!(f, [hard_line_break()])?;
            } else if separator_has_inline_whitespace {
                write_tree_jsx_whitespace_separator(f)?;
            } else {
                write!(f, [token("")])?;
            }

            Ok(())
        });

        fill.entry(
            &separator,
            &format_with(|f| write_tree_expression_argument(f, *element_id, None)),
        );
    }

    fill.finish()?;

    if suffix_spacing.0 {
        write!(f, [hard_line_break()])?;
    } else if suffix_spacing.1 {
        write_tree_jsx_whitespace_separator(f)?;
    } else if suffix_has_adjacent_newline {
        write!(f, [hard_line_break()])?;
    } else if suffix_has_adjacent_inline_whitespace {
        write_tree_jsx_whitespace_separator(f)?;
    }

    Ok(())
}

/// Format tree children using the selected layout rules.
fn format_tree_children<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
    layout: (bool, bool, bool, bool),
) -> FormatResult<()> {
    let (all_tree_children, only_tree_or_comment_children, force_break, force_break_with_fill) =
        layout;

    if force_break && !force_break_with_fill {
        return format_tree_children_multiline(f, elements);
    }

    if (all_tree_children || only_tree_or_comment_children) && elements.len() > 1 {
        return format_tree_children_tree_per_line(f, elements);
    }

    format_tree_children_fill(f, elements)
}

/// Return opening-tag layout booleans for one tree literal.
fn tree_opening_tag_layout(
    context: &DestackFormatContext<'_>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> (bool, bool) {
    let tag_has_static_type_arguments = !generic_arguments.is_empty();
    let single_attribute_per_line = context.options.single_attribute_per_line;
    let prefer_same_line_self_closing = !context.options.language_type.is_destack()
        && elements.is_none()
        && !single_attribute_per_line
        && arguments
            .as_ref()
            .is_some_and(|arguments| arguments.len() > 2)
        && !tag_has_static_type_arguments;
    let bracket_same_line = context.options.bracket_same_line || prefer_same_line_self_closing;

    (single_attribute_per_line, bracket_same_line)
}

/// Collect top-level layout data for one tree literal.
fn tree_literal_layout(
    context: &DestackFormatContext<'_>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> (bool, Option<(bool, bool, bool, bool)>, bool, bool) {
    let force_break_attributes = arguments
        .as_ref()
        .is_some_and(|arguments| should_force_break_tree_attributes(context, arguments));
    let children = elements.as_ref().and_then(|elements| {
        if elements.is_empty() {
            None
        } else {
            Some(tree_children_layout(
                context,
                elements,
                force_break_attributes,
            ))
        }
    });
    let has_multiline_whitespace_separator = elements
        .as_ref()
        .is_some_and(|elements| tree_literal_has_multiline_whitespace_separator(context, elements));
    let should_break = children
        .map(|(_, _, force_break, _)| force_break)
        .unwrap_or(force_break_attributes);
    let requires_expanded_layout = should_break || has_multiline_whitespace_separator;

    (
        force_break_attributes,
        children,
        should_break,
        requires_expanded_layout,
    )
}

/// Decide whether a tree literal should break across multiple lines.
pub(crate) fn tree_literal_should_break(
    context: &DestackFormatContext<'_>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> bool {
    tree_literal_layout(context, arguments, elements).2
}

/// Return whether a tree literal is the body of one lambda declaration.
fn tree_literal_is_lambda_body(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent_by_id(node_id.id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    matches!(
        context.tree.get(declaration_id),
        Declaration::Function(function)
            if function.signature.kind == FunctionKind::Lambda
                && function.body.is_some_and(|body_id| body_id == node_id)
    )
}

/// Return whether a tree literal should be wrapped in parentheses when it breaks.
pub(crate) fn tree_literal_wraps_on_break(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    // top-level expression statements stay unwrapped
    if expression_is_in_statement_position(context, node_id)
        && !tree_literal_is_lambda_body(context, node_id)
    {
        return false;
    }

    let Some((parent_id, parent_type)) = context.parent_by_id(node_id.id) else {
        return true;
    };

    match parent_type {
        NodeType::Expression => {
            let parent_id = LocalNodeId::<Expression>::new(parent_id);
            match context.tree.get(parent_id) {
                // jsx-like containers and conditional branches keep children unwrapped
                Expression::ArrayExpression { .. }
                | Expression::TupleExpression { .. }
                | Expression::TreeExpression { .. }
                | Expression::If {
                    kind: IfKind::Ternary,
                    ..
                } => false,
                // declaration expressions own their initializer grouping
                Expression::Let { .. } => false,
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
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
            let is_lambda_body = matches!(
                context.tree.get(declaration_id),
                Declaration::Function(function)
                    if function.signature.kind == FunctionKind::Lambda
                        && function.body.is_some_and(|body_id| body_id.id == node_id.id)
            );

            if is_lambda_body {
                return true;
            }

            true
        }
        NodeType::Declarator => true,
        _ => true,
    }
}

/// Return whether a tree literal should force expanded layout in one parent chain.
fn tree_literal_should_expand_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id.id;
    let mut current_type = NodeType::Expression;
    let mut is_lambda_body = false;
    let mut is_inside_call = false;

    loop {
        // current expression
        if current_type == NodeType::Expression {
            let current_expression_id = LocalNodeId::<Expression>::new(current_id);
            match context.tree.get(current_expression_id) {
                Expression::Call { .. } | Expression::New { .. } => {
                    if is_lambda_body {
                        is_inside_call = true;
                    }
                }
                Expression::TreeExpression { .. } => {
                    if current_id != node_id.id {
                        return is_inside_call;
                    }
                }
                _ => {}
            }
        }

        // parent edge
        let Some((parent_id, parent_type)) = context.parent_by_id(current_id) else {
            return false;
        };

        // lambda body owner
        if parent_type == NodeType::Declaration {
            if current_type != NodeType::Expression {
                return false;
            }

            let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
            let Declaration::Function(function) = context.tree.get(declaration_id) else {
                return false;
            };
            if function.signature.kind != FunctionKind::Lambda
                || !function
                    .body
                    .is_some_and(|body_id| body_id.id == current_id)
            {
                return false;
            }

            is_lambda_body = true;
            current_id = parent_id;
            current_type = parent_type;
            continue;
        }

        // continue through expression and argument wrappers
        if matches!(parent_type, NodeType::Expression | NodeType::Argument) {
            current_id = parent_id;
            current_type = parent_type;
            continue;
        }

        return false;
    }
}

/// Return the source span for the rendered tag body.
fn tree_literal_tag_span(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Span {
    context
        .tree
        .get_head_span(node_id)
        .unwrap_or_else(|| context.span(node_id))
}

/// Return whether conditional branch trailing comments were written for one tree literal.
fn write_tree_literal_conditional_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if f.context().options.language_type.is_destack() {
        return Ok(false);
    }

    let Some((parent_id, parent_type)) = f.context().parent_by_id(node_id.id) else {
        return Ok(false);
    };
    if parent_type != NodeType::Expression {
        return Ok(false);
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::If {
        kind: IfKind::Ternary,
        then_expression,
        else_expression,
        ..
    } = f.context().tree.get(parent_id)
    else {
        return Ok(false);
    };

    // alternate branch interior
    let comments = if else_expression
        .as_ref()
        .is_some_and(|else_expression| *else_expression == node_id)
    {
        let parent_span = f.context().span(parent_id);
        f.context().comments().comments_before(parent_span.end)
    }
    // consequent line suffix
    else if *then_expression == node_id {
        let node_span = tree_literal_tag_span(f.context(), node_id);
        f.context()
            .comments()
            .end_of_line_comments_after(node_span.end)
    }
    // other branch positions use default trailing comments
    else {
        return Ok(false);
    };

    write!(f, [FormatTrailingComments::Comments(comments)])?;

    Ok(true)
}

/// Write trailing comments owned by one tree literal.
fn write_tree_literal_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if write_tree_literal_conditional_trailing_comments(f, node_id)? {
        return Ok(());
    }

    Ok(())
}

/// Format one tree literal layout and its branch trailing comments.
fn format_tree_literal_with_branch_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
    layout: (bool, Option<(bool, bool, bool, bool)>, bool, bool),
) -> FormatResult<()> {
    let expression_span = f.context().span(node_id);
    let token_start = f.context().expression_token_start(node_id);
    let token_start_span = Span::new(expression_span.file, token_start, token_start);

    // leading comments
    write!(f, [format_leading_comments(token_start_span)])?;

    format_tree_literal_with_layout(
        f,
        node_id,
        left,
        generic_arguments,
        arguments,
        elements,
        layout,
    )?;
    write_tree_literal_trailing_comments(f, node_id)
}

/// Format a tree literal expression with optional wrap-on-break parentheses.
pub(crate) fn format_tree_literal_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
    let should_expand_in_parent = tree_literal_should_expand_in_parent(f.context(), node_id);
    let layout = tree_literal_layout(f.context(), arguments, elements);

    if !tree_literal_wraps_on_break(f.context(), node_id) {
        if !should_expand_in_parent {
            return format_tree_literal_with_branch_comments(
                f,
                node_id,
                left,
                generic_arguments,
                arguments,
                elements,
                layout,
            );
        }

        let formatted_tree = format_with(|f| {
            format_tree_literal_with_branch_comments(
                f,
                node_id,
                left,
                generic_arguments,
                arguments,
                elements,
                layout,
            )
        });

        return write!(f, [group(&formatted_tree).should_expand(true)]);
    }

    let should_expand = layout.3 || should_expand_in_parent;

    write!(
        f,
        [group(&format_with(|f| {
            write!(f, [if_group_breaks(&token("("))])?;

            let formatted_tree = format_with(|f| {
                format_tree_literal_with_branch_comments(
                    f,
                    node_id,
                    left,
                    generic_arguments,
                    arguments,
                    elements,
                    layout,
                )
            });
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

/// Format tree attributes in one opening tag.
fn format_tree_attributes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<Argument>],
    force_break_attributes: bool,
    single_attribute_per_line: bool,
    bracket_same_line: bool,
) -> FormatResult<()> {
    let attr_separator: &dyn Format<DestackFormatContext<'ast>> =
        if force_break_attributes || (single_attribute_per_line && arguments.len() > 1) {
            &hard_line_break()
        } else {
            &soft_line_break_or_space()
        };
    let format_attrs = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let following_span_starts = arguments
            .iter()
            .enumerate()
            .map(|(index, _)| {
                arguments
                    .get(index + 1)
                    .map(|argument| f.context().span(*argument).start)
            })
            .collect::<Vec<_>>();

        f.join_with(attr_separator)
            .entries(arguments.iter().enumerate().map(|(index, argument)| {
                let argument_id = *argument;
                let following_span_start = following_span_starts[index];

                format_with(move |f| {
                    write_tree_expression_argument(f, argument_id, following_span_start)
                })
            }))
            .finish()
    });

    if force_break_attributes {
        write!(f, [expand_parent()])?;
    }

    if bracket_same_line {
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
    } else if force_break_attributes {
        write!(
            f,
            [
                if_group_fits_on_line(&space()),
                group(&soft_block_indent(&format_attrs)).should_expand(true)
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

    Ok(())
}

/// Write one self-closing marker for a tree opening tag.
fn write_tree_self_closing_marker<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: &Option<LocalNodeId<Expression>>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    bracket_same_line: bool,
) -> FormatResult<()> {
    let has_attributes = arguments.is_some();
    if left.is_some() || has_attributes {
        if has_attributes {
            if bracket_same_line {
                write!(f, [if_group_breaks(&space())])?;
            }
            write!(f, [if_group_fits_on_line(&space())])?;
        } else {
            write!(f, [space()])?;
        }
    }

    write!(f, [token("/")])
}

/// Format one tree opening tag.
fn format_tree_opening_tag<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: &Option<LocalNodeId<Expression>>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
    layout: (bool, Option<(bool, bool, bool, bool)>, bool, bool),
) -> FormatResult<()> {
    let (single_attribute_per_line, bracket_same_line) =
        tree_opening_tag_layout(f.context(), generic_arguments, arguments, elements);
    let force_break_attributes = layout.0;

    write!(f, [token("<")])?;
    if let Some(left) = left {
        write!(f, [left])?;
    }
    if !generic_arguments.is_empty() {
        format_generic_argument_list(f, generic_arguments)?;
    }

    if let Some(arguments) = arguments {
        format_tree_attributes(
            f,
            arguments,
            force_break_attributes,
            single_attribute_per_line,
            bracket_same_line,
        )?;
    }

    if elements.is_none() {
        write_tree_self_closing_marker(f, left, arguments, bracket_same_line)?;
    }

    write!(f, [token(">")])
}

/// Format one tree body and closing tag.
fn format_tree_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
    layout: (bool, Option<(bool, bool, bool, bool)>, bool, bool),
) -> FormatResult<()> {
    let Some(elements) = elements else {
        return Ok(());
    };

    if elements.is_empty() {
        write!(f, [block_infix_annotations(f.context(), expression_id)])?;
        return write_tree_closing_tag(f, expression_id, left);
    }

    let force_break_attributes = layout.0;
    let children_layout = layout
        .1
        .unwrap_or_else(|| tree_children_layout(f.context(), elements, force_break_attributes));
    let format_children = format_with(|f| format_tree_children(f, elements, children_layout));
    if children_layout.2 {
        write!(f, [block_indent(&group(&format_children))])?;
    } else {
        write!(f, [group(&soft_block_indent(&format_children))])?;
    }

    write!(f, [block_infix_annotations(f.context(), expression_id)])?;
    write_tree_closing_tag(f, expression_id, left)
}

/// Format one tree literal from precomputed layout data.
fn format_tree_literal_with_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
    layout: (bool, Option<(bool, bool, bool, bool)>, bool, bool),
) -> FormatResult<()> {
    let should_expand = layout.3;

    write!(
        f,
        [group(&format_with(|f| {
            let opening_tag = format_with(|f| {
                format_tree_opening_tag(f, left, generic_arguments, arguments, elements, layout)
            });
            write!(f, [group(&opening_tag)])?;
            format_tree_body(f, _expression_id, left, elements, layout)
        }))
        .should_expand(should_expand)]
    )
}
