use crate::annotation::{FormatTrailingComments, block_infix_annotations, format_leading_comments};
use crate::chain::transparent_inner_expression;
use crate::context::MemoizeFormatExt;
use crate::declaration::expression_is_in_statement_position;
use crate::tree::{
    FormatTreeOpeningElement, is_jsx_whitespace_char, should_force_break_tree_attributes,
    tree_argument_is_wrapped_in_braces, tree_child_breaks_element,
    tree_children_have_blank_line_between, tree_text_is_whitespace_only,
    write_tree_expression_argument,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Declaration, Expression, FunctionKind, GenericArgument, IfKind, LocalNodeId,
    NodeType, ScalarLiteral, Tree,
};
use destack_fir::format::{Buffer, FormatNodes, FormatResult};
use destack_fir::prelude::{
    block_indent, empty_line, format_with, group, hard_line_break, if_group_breaks,
    if_group_fits_on_line, soft_block_indent, soft_line_break, soft_line_break_or_space, space,
    text, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::QuoteStyle;

/// Return the value expression id for one tree child argument.
fn tree_child_value_id(
    tree: &Tree,
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
        if let Some((is_whitespace_only, _)) = whitespace_info {
            if !is_whitespace_only {
                has_non_whitespace_text_child = true;
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
    let force_break =
        // tag children follow jsx child-list layout
        force_break_attributes
            || (has_breaking_child && elements.len() > 1)
            || has_tree_child
            || has_multiple_expression_children;
    let force_break_with_fill = force_break
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

/// One split tree child used by the JSX child-list fill layout.
#[derive(Clone, Debug)]
enum TreeSplitChild {
    /// One text word.
    Word(String),
    /// One JSX whitespace separator.
    Whitespace,
    /// One source newline separator.
    Newline,
    /// One source empty-line separator.
    EmptyLine,
    /// One non-text child argument.
    NonText(LocalNodeId<Argument>),
}

/// One tree text chunk.
#[derive(Clone, Copy, Debug)]
enum TreeTextChunk<'a> {
    /// A whitespace run.
    Whitespace(&'a str),
    /// A non-whitespace word.
    Word(&'a str),
}

/// Split one text child into JSX text chunks.
fn tree_text_chunks(text: &str) -> Vec<TreeTextChunk<'_>> {
    let mut chunks = Vec::new();
    let mut chunk_start = 0usize;
    let mut chunk_is_whitespace = None::<bool>;

    for (index, character) in text.char_indices() {
        let is_whitespace = is_jsx_whitespace_char(character);
        let Some(previous_is_whitespace) = chunk_is_whitespace else {
            chunk_is_whitespace = Some(is_whitespace);
            continue;
        };

        if previous_is_whitespace == is_whitespace {
            continue;
        }

        let chunk = &text[chunk_start..index];
        if previous_is_whitespace {
            chunks.push(TreeTextChunk::Whitespace(chunk));
        } else {
            chunks.push(TreeTextChunk::Word(chunk));
        }

        chunk_start = index;
        chunk_is_whitespace = Some(is_whitespace);
    }

    let Some(is_whitespace) = chunk_is_whitespace else {
        return chunks;
    };
    let chunk = &text[chunk_start..];
    if is_whitespace {
        chunks.push(TreeTextChunk::Whitespace(chunk));
    } else {
        chunks.push(TreeTextChunk::Word(chunk));
    }

    chunks
}

/// Push one split JSX child with whitespace coalescing rules.
fn push_tree_split_child(children: &mut Vec<TreeSplitChild>, child: TreeSplitChild) {
    match children.last_mut() {
        Some(
            last @ (TreeSplitChild::EmptyLine
            | TreeSplitChild::Newline
            | TreeSplitChild::Whitespace),
        ) => {
            if matches!(child, TreeSplitChild::Whitespace) {
                *last = child;
            } else if matches!(child, TreeSplitChild::NonText(_) | TreeSplitChild::Word(_)) {
                children.push(child);
            }
        }
        _ => children.push(child),
    }
}

/// Return whether one child is a comment-free JSX whitespace expression.
fn tree_argument_is_jsx_whitespace_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !tree_argument_is_wrapped_in_braces(context, argument_id) {
        return false;
    }

    if context
        .comments()
        .has_comment_in_span(context.span(argument_id))
    {
        return false;
    }

    let Argument::Positional { value, .. } = context.tree.get(argument_id) else {
        return false;
    };
    let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = context.tree.get(*value)
    else {
        return false;
    };

    context.strings.get(*string_id) == " "
}

/// Split tree children with JSX child-list rules.
fn split_tree_children(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
) -> Vec<TreeSplitChild> {
    let mut children = Vec::new();

    for element_id in elements {
        if tree_argument_is_jsx_whitespace_expression(context, *element_id) {
            push_tree_split_child(&mut children, TreeSplitChild::Whitespace);
            continue;
        }

        let Argument::Positional { value, .. } = context.tree.get(*element_id) else {
            push_tree_split_child(&mut children, TreeSplitChild::NonText(*element_id));
            continue;
        };
        if tree_argument_is_wrapped_in_braces(context, *element_id) {
            push_tree_split_child(&mut children, TreeSplitChild::NonText(*element_id));
            continue;
        }

        let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = context.tree.get(*value)
        else {
            push_tree_split_child(&mut children, TreeSplitChild::NonText(*element_id));
            continue;
        };

        let mut chunks = tree_text_chunks(context.strings.get(*string_id))
            .into_iter()
            .peekable();
        if let Some(TreeTextChunk::Whitespace(_)) = chunks.peek() {
            let Some(TreeTextChunk::Whitespace(whitespace)) = chunks.next() else {
                unreachable!("peeked whitespace chunk should be whitespace");
            };

            if whitespace.contains('\n') {
                if chunks.peek().is_none() {
                    let newline_count = whitespace.bytes().filter(|byte| *byte == b'\n').count();
                    if newline_count > 1 {
                        push_tree_split_child(&mut children, TreeSplitChild::EmptyLine);
                    }

                    continue;
                }

                push_tree_split_child(&mut children, TreeSplitChild::Newline);
            } else {
                push_tree_split_child(&mut children, TreeSplitChild::Whitespace);
            }
        }

        while let Some(chunk) = chunks.next() {
            match chunk {
                TreeTextChunk::Word(word) => {
                    push_tree_split_child(&mut children, TreeSplitChild::Word(word.to_string()));
                }
                TreeTextChunk::Whitespace(whitespace) => {
                    if chunks.peek().is_none() {
                        if whitespace.contains('\n') {
                            push_tree_split_child(&mut children, TreeSplitChild::Newline);
                        } else {
                            push_tree_split_child(&mut children, TreeSplitChild::Whitespace);
                        }
                    }
                }
            }
        }
    }

    if matches!(
        children.last(),
        Some(TreeSplitChild::EmptyLine | TreeSplitChild::Newline)
    ) {
        children.pop();
    }

    if matches!(
        children.first(),
        Some(TreeSplitChild::EmptyLine | TreeSplitChild::Newline)
    ) {
        children.remove(0);
    }

    children
}

/// Separator before one split tree child in fill layout.
#[derive(Clone, Copy, Debug)]
enum TreeSplitSeparator {
    /// No separator.
    None,
    /// JSX whitespace.
    Whitespace,
    /// Soft word separator.
    SoftOrSpace,
    /// Soft child separator.
    Soft,
    /// Hard line break.
    Hard,
    /// Empty line.
    Empty,
}

/// Return the separator before one visible split child.
fn tree_split_separator(
    previous_visible: Option<&TreeSplitChild>,
    pending_separator: TreeSplitSeparator,
    current: &TreeSplitChild,
) -> TreeSplitSeparator {
    if !matches!(pending_separator, TreeSplitSeparator::None) {
        return pending_separator;
    }

    match (previous_visible, current) {
        (Some(TreeSplitChild::Word(_)), TreeSplitChild::Word(_)) => TreeSplitSeparator::SoftOrSpace,
        (Some(TreeSplitChild::Word(_)), TreeSplitChild::NonText(_))
        | (Some(TreeSplitChild::NonText(_)), TreeSplitChild::Word(_)) => TreeSplitSeparator::Soft,
        (Some(TreeSplitChild::NonText(_)), TreeSplitChild::NonText(_)) => TreeSplitSeparator::Hard,
        _ => TreeSplitSeparator::None,
    }
}

/// Format one split child separator.
fn write_tree_split_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    separator: TreeSplitSeparator,
) -> FormatResult<()> {
    match separator {
        TreeSplitSeparator::None => {}
        TreeSplitSeparator::Whitespace => write_tree_jsx_whitespace_separator(f)?,
        TreeSplitSeparator::SoftOrSpace => write!(f, [soft_line_break_or_space()])?,
        TreeSplitSeparator::Soft => write!(f, [soft_line_break()])?,
        TreeSplitSeparator::Hard => write!(f, [hard_line_break()])?,
        TreeSplitSeparator::Empty => write!(f, [empty_line()])?,
    }

    Ok(())
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

/// Return the single template child that stays attached to its enclosing tags.
fn tree_literal_single_template_child(
    context: &DestackFormatContext<'_>,
    elements: &[LocalNodeId<Argument>],
) -> Option<LocalNodeId<Argument>> {
    let [argument_id] = elements else {
        return None;
    };
    let value_id = tree_child_value_id(context.tree, *argument_id)?;
    let value_id = transparent_inner_expression(context, value_id);
    let value = context.tree.get(value_id);

    if matches!(
        value,
        Expression::TemplateExpression { .. } | Expression::TaggedTemplateExpression { .. }
    ) {
        Some(*argument_id)
    } else {
        None
    }
}

/// Format mixed tree children with fill separators.
fn format_tree_children_fill<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<Argument>],
    force_multiline: bool,
) -> FormatResult<()> {
    let children = split_tree_children(f.context(), elements);
    if children.is_empty() {
        let (has_newline_spacing, has_inline_spacing) =
            tree_whitespace_run_spacing(f.context(), elements);
        if has_newline_spacing {
            write!(f, [hard_line_break()])?;
        } else if has_inline_spacing {
            write_tree_jsx_whitespace_separator(f)?;
        }

        return Ok(());
    }

    let mut fill = f.fill();
    let mut previous_visible = None::<TreeSplitChild>;
    let mut pending_separator = TreeSplitSeparator::None;

    for child in &children {
        match child {
            TreeSplitChild::Whitespace => {
                if force_multiline {
                    pending_separator = TreeSplitSeparator::Whitespace;
                    continue;
                }

                let separator =
                    tree_split_separator(previous_visible.as_ref(), pending_separator, child);
                let separator = format_with(|f| write_tree_split_separator(f, separator));
                let whitespace = format_with(write_tree_jsx_whitespace_separator);
                fill.entry(&separator, &whitespace);
                previous_visible = Some(child.clone());
                pending_separator = TreeSplitSeparator::None;
            }
            TreeSplitChild::Newline => {
                pending_separator = TreeSplitSeparator::Hard;
            }
            TreeSplitChild::EmptyLine => {
                pending_separator = TreeSplitSeparator::Empty;
            }
            TreeSplitChild::Word(word) => {
                let separator =
                    tree_split_separator(previous_visible.as_ref(), pending_separator, child);
                let separator = format_with(|f| write_tree_split_separator(f, separator));
                fill.entry(&separator, &text(word.as_str()));
                previous_visible = Some(child.clone());
                pending_separator = TreeSplitSeparator::None;
            }
            TreeSplitChild::NonText(element_id) => {
                let separator =
                    tree_split_separator(previous_visible.as_ref(), pending_separator, child);
                let separator = format_with(|f| write_tree_split_separator(f, separator));
                fill.entry(
                    &separator,
                    &format_with(|f| write_tree_expression_argument(f, *element_id, None)),
                );
                previous_visible = Some(child.clone());
                pending_separator = TreeSplitSeparator::None;
            }
        }
    }

    fill.finish()
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

    format_tree_children_fill(f, elements, force_break)
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
pub(crate) fn tree_literal_uses_conditional_trailing_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent_by_id(node_id.id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::If {
        kind: IfKind::Ternary,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(parent_id)
    else {
        return false;
    };

    *then_expression == node_id || else_expression.as_ref().is_some_and(|id| *id == node_id)
}

/// Return whether conditional branch trailing comments were written for one tree literal.
fn write_tree_literal_conditional_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    if !tree_literal_uses_conditional_trailing_comments(f.context(), node_id) {
        return Ok(false);
    }

    let Some((parent_id, _)) = f.context().parent_by_id(node_id.id) else {
        return Ok(false);
    };
    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::If {
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

/// Format one tree body and closing tag.
fn format_tree_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
    force_multiline_children: bool,
) -> FormatResult<()> {
    let Some(elements) = elements else {
        return Ok(());
    };

    if elements.is_empty() {
        write!(f, [block_infix_annotations(f.context(), expression_id)])?;
        return write_tree_closing_tag(f, expression_id, left);
    }

    if let Some(template_child) = tree_literal_single_template_child(f.context(), elements) {
        write!(
            f,
            [format_with(|f| write_tree_expression_argument(
                f,
                template_child,
                None
            ))]
        )?;
        write!(f, [block_infix_annotations(f.context(), expression_id)])?;
        return write_tree_closing_tag(f, expression_id, left);
    }

    let children_layout = tree_children_layout(f.context(), elements, force_multiline_children);

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
            let opening_tag = FormatTreeOpeningElement::new(
                _expression_id,
                left,
                generic_arguments,
                arguments,
                elements,
                layout.0,
            )
            .memoized();
            let opening_breaks = opening_tag
                .inspect(f)?
                .is_some_and(|opening_tag| opening_tag.will_break());
            let multiple_attributes = arguments
                .as_ref()
                .is_some_and(|arguments| arguments.len() > 1);
            let force_multiline_children = multiple_attributes || opening_breaks || layout.0;

            write!(f, [group(&opening_tag)])?;
            format_tree_body(f, _expression_id, left, elements, force_multiline_children)
        }))
        .should_expand(should_expand)]
    )
}
