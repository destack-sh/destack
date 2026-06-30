use super::text::{
    format_tree_children_inline_fill, tree_child_allows_trailing_inline_punctuation,
    tree_text_is_inline_punctuation, write_tree_text_words,
};
use crate::annotation::{block_infix_annotations, format_leading_comments, write_comment_slice};
use crate::chain::{argument_value_id_if_present, transparent_inner_expression};
use crate::context::PreparedFormat;
use crate::declaration::expression_is_in_statement_context;
use crate::expression::ternary_branch_trailing_comments;
use crate::tree::{
    FormatTreeOpeningElement, should_force_break_tree_attributes, tree_child_breaks_element,
    tree_children_have_blank_line_between, tree_text_child_text, tree_text_is_whitespace_only,
    write_tree_child,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_dir::{
    Argument, Comment, Declaration, Expression, FunctionForm, GenericArgument, IfForm, LocalNodeId,
    NodeType, ScalarLiteral, Tree, TreeAttribute, TreeChild,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    block_indent, empty_line, format_with, group, hard_line_break, if_group_breaks,
    soft_block_indent, token,
};
use destack_fir::write;
use destack_source::Span;
use smallvec::SmallVec;

/// Return the value expression id for one tree child.
fn tree_child_value_id(
    tree: &Tree,
    child_id: LocalNodeId<TreeChild>,
) -> Option<LocalNodeId<Expression>> {
    tree.get(child_id).value()
}

/// Build tree-child layout data for one tree body.
fn tree_children_layout(
    context: &DestackFormatContext<'_>,
    children: &[LocalNodeId<TreeChild>],
    force_break_attributes: bool,
) -> TreeChildrenLayout {
    let tree = context.tree;
    let mut tree_child_count = 0usize;
    let mut expression_child_count = 0usize;
    let mut has_breaking_child = false;
    let mut has_non_whitespace_text_child = false;
    let mut only_tree_or_comment_children = true;

    for child_id in children {
        let whitespace_info = tree_text_is_whitespace_only(context, *child_id);
        if let Some((is_whitespace_only, _)) = whitespace_info
            && !is_whitespace_only
        {
            has_non_whitespace_text_child = true;
        }

        // tree per line layout ignores pure whitespace text separators
        let is_non_content_text_separator =
            whitespace_info.is_some_and(|(is_whitespace_only, _)| is_whitespace_only);

        let value_id = tree_child_value_id(tree, *child_id);
        let Some(value_id) = value_id else {
            if !matches!(tree.get(*child_id), TreeChild::Text { .. })
                && !is_non_content_text_separator
            {
                expression_child_count += 1;
                only_tree_or_comment_children = false;
            } else if !is_non_content_text_separator {
                only_tree_or_comment_children = false;
            }
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

        if tree_child_breaks_element(context, *child_id) {
            has_breaking_child = true;
        }
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
    let tree_should_break_mixed_text = has_tree_and_text_children
        && (context.options.language_type.is_destack() || tree_child_count > 1);
    let force_break =
        // tag children follow jsx child-list layout
        force_break_attributes
            || (has_breaking_child && children.len() > 1)
            || (has_tree_child && !has_non_whitespace_text_child)
            || tree_should_break_mixed_text
            || (has_multiple_expression_children && !has_non_whitespace_text_child);
    let force_break_with_fill = force_break
        && has_tree_and_text_children
        && expression_child_count == 0
        && !has_tree_and_expression_children;

    TreeChildrenLayout {
        all_tree_children: tree_child_count == children.len(),
        only_tree_or_comment_children,
        force_break,
        force_break_with_fill,
    }
}

/// Layout policy for one tree child list.
#[derive(Clone, Copy, Debug)]
struct TreeChildrenLayout {
    /// Whether every child is a tree expression.
    all_tree_children: bool,
    /// Whether every visible child is a tree expression or comment stub.
    only_tree_or_comment_children: bool,
    /// Whether the child list must break.
    force_break: bool,
    /// Whether forced breaks should still use JSX fill layout.
    force_break_with_fill: bool,
}

impl TreeChildrenLayout {
    /// Return whether children should use one-child-per-line layout.
    fn should_format_multiline(self) -> bool {
        self.force_break && !self.force_break_with_fill
    }

    /// Return whether children should use tree-per-line layout.
    fn should_format_tree_per_line(self, child_count: usize) -> bool {
        (self.all_tree_children || self.only_tree_or_comment_children) && child_count > 1
    }
}

/// Top-level layout policy for one tree literal.
#[derive(Clone, Copy, Debug)]
struct TreeLiteralLayout {
    /// Whether attributes force the opening element to break.
    force_break_attributes: bool,
    /// Whether the literal should break.
    should_break: bool,
    /// Whether the literal should expand its parent group.
    requires_expanded_layout: bool,
}

/// Write one tree closing tag.
fn write_tree_closing_tag<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    write!(f, [token("</")])?;
    if let Some(left) = left {
        write!(f, [left])?;
    }
    write!(f, [token(">")])?;
    Ok(())
}

/// Format tree children in one-child-per-line mode.
fn format_tree_children_multiline<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    children: &[LocalNodeId<TreeChild>],
) -> FormatResult<()> {
    let mut wrote_child = false;
    let mut pending_blank_line = false;
    let mut previous_emitted_child: Option<LocalNodeId<TreeChild>> = None;

    for child_id in children {
        let whitespace_info = tree_text_is_whitespace_only(f.context(), *child_id);
        let is_whitespace_only =
            whitespace_info.is_some_and(|(is_whitespace_only, _)| is_whitespace_only);
        let has_blank_line = whitespace_info.is_some_and(|(_, has_blank_line)| has_blank_line);

        if is_whitespace_only {
            if has_blank_line && wrote_child {
                pending_blank_line = true;
            }
            continue;
        }

        let text = tree_text_child_text(f.context(), *child_id).map(str::to_owned);
        let is_inline_punctuation = text.as_deref().is_some_and(tree_text_is_inline_punctuation);
        let previous_allows_inline_punctuation =
            previous_emitted_child.is_some_and(|previous_child_id| {
                tree_child_allows_trailing_inline_punctuation(f.context(), previous_child_id)
            });
        let has_blank_line_between_children =
            previous_emitted_child.is_some_and(|previous_child_id| {
                tree_children_have_blank_line_between(f.context(), previous_child_id, *child_id)
            });
        let should_attach_inline_punctuation = is_inline_punctuation
            && previous_allows_inline_punctuation
            && !pending_blank_line
            && !has_blank_line_between_children;

        if wrote_child && !should_attach_inline_punctuation {
            if pending_blank_line || has_blank_line_between_children {
                write!(f, [empty_line()])?;
                pending_blank_line = false;
            } else {
                write!(f, [hard_line_break()])?;
            }
        }

        let wrote_text = if let Some(text) = text.as_deref() {
            write_tree_text_words(f, text)?
        } else {
            false
        };

        if !wrote_text {
            write!(f, [format_with(|f| write_tree_child(f, *child_id, None))])?;
        }
        wrote_child = true;
        previous_emitted_child = Some(*child_id);
    }

    Ok(())
}

/// Format tree children with one tree child per line.
fn format_tree_children_tree_per_line<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    children: &[LocalNodeId<TreeChild>],
) -> FormatResult<()> {
    for (index, child_id) in children.iter().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [format_with(|f| write_tree_child(f, *child_id, None))])?;
    }

    Ok(())
}

/// Return whether one tree child is a multiline tree expression in source.
fn tree_child_is_multiline_tree_expression(
    context: &DestackFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
) -> bool {
    let Some(value_id) = tree_child_value_id(context.tree, child_id) else {
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
    children: &[LocalNodeId<TreeChild>],
) -> bool {
    children.iter().enumerate().any(|(index, child_id)| {
        let is_braced_whitespace =
            matches!(context.tree.get(*child_id), TreeChild::Expression { .. })
                && tree_text_is_whitespace_only(context, *child_id)
                    .is_some_and(|(is_whitespace_only, _)| is_whitespace_only);
        if !is_braced_whitespace {
            return false;
        }

        let previous_is_multiline_tree = index
            .checked_sub(1)
            .and_then(|previous_index| children.get(previous_index).copied())
            .is_some_and(|child_id| tree_child_is_multiline_tree_expression(context, child_id));
        let next_is_multiline_tree = children
            .get(index + 1)
            .copied()
            .is_some_and(|child_id| tree_child_is_multiline_tree_expression(context, child_id));

        previous_is_multiline_tree || next_is_multiline_tree
    })
}

/// Return the single template child that stays attached to its enclosing tags.
fn tree_literal_single_template_child(
    context: &DestackFormatContext<'_>,
    children: &[LocalNodeId<TreeChild>],
) -> Option<LocalNodeId<TreeChild>> {
    let [child_id] = children else {
        return None;
    };
    let value_id = tree_child_value_id(context.tree, *child_id)?;
    let value_id = transparent_inner_expression(context, value_id);
    let value = context.tree.get(value_id);

    if matches!(
        value,
        Expression::TemplateExpression { .. } | Expression::TaggedTemplateExpression { .. }
    ) {
        Some(*child_id)
    } else {
        None
    }
}

/// Format tree children using the selected layout rules.
fn format_tree_children<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    children: &[LocalNodeId<TreeChild>],
    layout: TreeChildrenLayout,
) -> FormatResult<()> {
    if layout.should_format_multiline() {
        return format_tree_children_multiline(f, children);
    }

    if layout.should_format_tree_per_line(children.len()) {
        return format_tree_children_tree_per_line(f, children);
    }

    format_tree_children_inline_fill(f, children, layout.force_break)
}

/// Collect top-level layout data for one tree literal.
fn tree_literal_layout(
    context: &DestackFormatContext<'_>,
    attributes: &Option<Vec<LocalNodeId<TreeAttribute>>>,
    children: &Option<Vec<LocalNodeId<TreeChild>>>,
) -> TreeLiteralLayout {
    let force_break_attributes = attributes
        .as_ref()
        .is_some_and(|attributes| should_force_break_tree_attributes(context, attributes));
    let child_layout = children.as_ref().and_then(|children| {
        if children.is_empty() {
            None
        } else {
            Some(tree_children_layout(
                context,
                children,
                force_break_attributes,
            ))
        }
    });
    let has_multiline_whitespace_separator = children
        .as_ref()
        .is_some_and(|children| tree_literal_has_multiline_whitespace_separator(context, children));
    let should_break = child_layout
        .map(|layout| layout.force_break)
        .unwrap_or(force_break_attributes);
    let requires_expanded_layout = should_break || has_multiline_whitespace_separator;

    TreeLiteralLayout {
        force_break_attributes,
        should_break,
        requires_expanded_layout,
    }
}

/// Decide whether a tree literal should break across multiple lines.
pub(crate) fn tree_literal_should_break(
    context: &DestackFormatContext<'_>,
    attributes: &Option<Vec<LocalNodeId<TreeAttribute>>>,
    children: &Option<Vec<LocalNodeId<TreeChild>>>,
) -> bool {
    tree_literal_layout(context, attributes, children).should_break
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
            if function.signature.form == FunctionForm::Lambda
                && function.body.is_some_and(|body_id| body_id == node_id)
    )
}

/// Return whether one tree literal is a call-like argument value.
fn tree_literal_is_call_argument(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    argument_id: u32,
) -> bool {
    let argument_id = LocalNodeId::<Argument>::new(argument_id);
    if argument_value_id_if_present(context.tree, argument_id) != Some(node_id) {
        return false;
    }

    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    matches!(
        context.tree.get(parent_id),
        Expression::Call { .. } | Expression::New { .. } | Expression::NewMaybe { .. }
    )
}

/// Return whether a tree literal should be wrapped in parentheses when it breaks.
pub(crate) fn tree_literal_wraps_on_break(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    // top-level expression statements stay unwrapped
    if expression_is_in_statement_context(context, node_id)
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
                    form: IfForm::Ternary,
                    ..
                } => false,
                // declaration expressions own their initializer grouping
                Expression::Let { .. } => false,
                // return handles jsx wrapping at the statement formatter level
                Expression::Return { .. } => false,
                _ => true,
            }
        }
        NodeType::Argument => !tree_literal_is_call_argument(context, node_id, parent_id),
        NodeType::TreeChild | NodeType::TreeAttribute => {
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
                    | Expression::NewMaybe { .. }
                    | Expression::ArrayExpression { .. }
                    | Expression::TupleExpression { .. }
                    | Expression::TreeExpression { .. }
                    | Expression::If {
                        form: IfForm::Ternary,
                        ..
                    }
            )
        }
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
            let is_lambda_body = matches!(
                context.tree.get(declaration_id),
                Declaration::Function(function)
                    if function.signature.form == FunctionForm::Lambda
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
    let should_expand_tree_callback_body = context.should_expand_tree_callback_bodies()
        && tree_literal_is_lambda_body(context, node_id);

    if should_expand_tree_callback_body {
        return true;
    }

    let mut current_id = node_id.id;
    let mut current_type = NodeType::Expression;
    let mut is_lambda_body = false;
    let mut is_inside_call = false;

    loop {
        // current expression
        if current_type == NodeType::Expression {
            let current_expression_id = LocalNodeId::<Expression>::new(current_id);
            match context.tree.get(current_expression_id) {
                Expression::Call { .. } | Expression::New { .. } | Expression::NewMaybe { .. } => {
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
            if !(function.signature.form == FunctionForm::Lambda
                && function
                    .body
                    .is_some_and(|body_id| body_id.id == current_id))
            {
                return false;
            }

            is_lambda_body = true;
            current_id = parent_id;
            current_type = parent_type;
            continue;
        }

        // continue through expression and tree wrappers
        if matches!(
            parent_type,
            NodeType::Expression | NodeType::TreeChild | NodeType::TreeAttribute
        ) {
            current_id = parent_id;
            current_type = parent_type;
            continue;
        }

        return false;
    }
}

/// Write ternary branch trailing comments attached to one tree literal.
fn write_tree_literal_ternary_branch_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let Some((_, comments)) = ternary_branch_trailing_comments(f.context(), node_id) else {
        return Ok(false);
    };

    let comments: SmallVec<[Comment; 2]> = comments.iter().copied().collect();
    write_comment_slice(f, comments.as_slice())?;

    Ok(true)
}

/// Write trailing comments owned by one tree literal.
fn write_tree_literal_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if write_tree_literal_ternary_branch_trailing_comments(f, node_id)? {
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
    attributes: &Option<Vec<LocalNodeId<TreeAttribute>>>,
    children: &Option<Vec<LocalNodeId<TreeChild>>>,
    layout: TreeLiteralLayout,
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
        attributes,
        children,
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
    attributes: &Option<Vec<LocalNodeId<TreeAttribute>>>,
    children: &Option<Vec<LocalNodeId<TreeChild>>>,
) -> FormatResult<()> {
    let should_expand_in_parent = tree_literal_should_expand_in_parent(f.context(), node_id);
    let layout = tree_literal_layout(f.context(), attributes, children);

    if !tree_literal_wraps_on_break(f.context(), node_id) {
        if !should_expand_in_parent {
            return format_tree_literal_with_branch_comments(
                f,
                node_id,
                left,
                generic_arguments,
                attributes,
                children,
                layout,
            );
        }

        let formatted_tree = format_with(|f| {
            format_tree_literal_with_branch_comments(
                f,
                node_id,
                left,
                generic_arguments,
                attributes,
                children,
                layout,
            )
        });

        return write!(f, [group(&formatted_tree).should_expand(true)]);
    }

    let should_expand = layout.requires_expanded_layout || should_expand_in_parent;

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
                    attributes,
                    children,
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
    children: &Option<Vec<LocalNodeId<TreeChild>>>,
    force_multiline_children: bool,
) -> FormatResult<()> {
    let Some(children) = children else {
        return Ok(());
    };

    if children.is_empty() {
        write!(f, [block_infix_annotations(f.context(), expression_id)])?;
        return write_tree_closing_tag(f, expression_id, left);
    }

    if let Some(template_child) = tree_literal_single_template_child(f.context(), children) {
        write!(
            f,
            [format_with(|f| write_tree_child(f, template_child, None))]
        )?;
        write!(f, [block_infix_annotations(f.context(), expression_id)])?;
        return write_tree_closing_tag(f, expression_id, left);
    }

    let children_layout = tree_children_layout(f.context(), children, force_multiline_children);

    let format_children = format_with(|f| format_tree_children(f, children, children_layout));
    if children_layout.force_break {
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
    attributes: &Option<Vec<LocalNodeId<TreeAttribute>>>,
    children: &Option<Vec<LocalNodeId<TreeChild>>>,
    layout: TreeLiteralLayout,
) -> FormatResult<()> {
    let should_expand = layout.requires_expanded_layout;

    write!(
        f,
        [group(&format_with(|f| {
            let opening_tag = PreparedFormat::new(
                f,
                FormatTreeOpeningElement::new(
                    _expression_id,
                    left,
                    generic_arguments,
                    attributes,
                    children,
                    layout.force_break_attributes,
                ),
            )?;
            let multiple_attributes = attributes
                .as_ref()
                .is_some_and(|attributes| attributes.len() > 1);
            let force_multiline_children = multiple_attributes || layout.force_break_attributes;

            write!(f, [group(&opening_tag)])?;
            format_tree_body(f, _expression_id, left, children, force_multiline_children)
        }))
        .should_expand(should_expand)]
    )
}
