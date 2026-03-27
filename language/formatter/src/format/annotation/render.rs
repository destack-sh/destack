use super::facts::token_type_is_comment_trivia;
use super::semicolon::annotation_needs_semicolon_guard_continuation_indent;
use crate::format::directive::{
    comment_node_is_any_ignore_directive, comment_node_is_ignore_directive,
    node_has_ignore_directive,
};
use crate::format::expression::format_expression;
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Blank, Comment, CommentStyle, Doc, DocStyle, Expression, LocalNodeId, Node,
    NodeTree, NodeTreeImpl, NodeType, TokenType,
};
use destack_fir::format::{Format, FormatResult, hard_line_break};
use destack_fir::prelude::*;
use destack_fir::write;

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    /// Format one annotation wrapper node.
    fn format_node(
        &self,
        node_id: LocalNodeId<Annotation>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Annotation::Blank { node, .. } => {
                // skip trailing blanks at the end of the source
                let annotation_span = f.context().annotation_span(node_id);
                let next_token_is_end = f
                    .context()
                    .annotation_next_non_whitespace_token_type(node_id)
                    == Some(TokenType::End);
                if next_token_is_end
                    || annotation_span.end >= f.context().file.len.saturating_sub(1)
                {
                    return Ok(());
                }

                node.format(f)
            }
            Annotation::Doc { node, .. } => node.format(f),
            Annotation::Comment { node, .. } => node.format(f),
            Annotation::Decorator { node, .. } => node.format(f),
        }
    }
}

impl<'ast> FormatNode<'ast, Blank> for Blank {
    /// Format one blank annotation node.
    fn format_node(
        &self,
        _node_id: LocalNodeId<Blank>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // reduce any number of blank lines to a single one
        write!(f, [empty_line()])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Doc> for Doc {
    /// Format one documentation comment annotation.
    fn format_node(
        &self,
        node_id: LocalNodeId<Doc>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let raw_comment = f.context().doc_raw_text(node_id);
        let is_block_comment = self.style == DocStyle::Star;
        format_comment_like_raw_text(f, raw_comment, is_block_comment)
    }
}

impl<'ast> FormatNode<'ast, Comment> for Comment {
    /// Format one comment annotation.
    fn format_node(
        &self,
        node_id: LocalNodeId<Comment>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let raw_comment = f.context().comment_raw_text(node_id);
        let is_block_comment = self.style == CommentStyle::Star;
        format_comment_like_raw_text(f, raw_comment, is_block_comment)
    }
}

impl<'ast> FormatNode<'ast, destack_ast::Decorator> for destack_ast::Decorator {
    /// Format one decorator annotation.
    fn format_node(
        &self,
        _node_id: LocalNodeId<destack_ast::Decorator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let tree = f.context().tree;
        let needs_parentheses = decorator_needs_parentheses(tree, self.expression);
        let expression_id = self.expression;
        let expression = tree.get(expression_id);
        let is_ignored = node_has_ignore_directive(f.context(), expression_id);

        write!(f, [token("@")])?;
        if needs_parentheses {
            write!(f, [token("(")])?;
        }

        format_expression(f, expression_id, expression, is_ignored)?;

        if needs_parentheses {
            write!(f, [token(")")])?;
        }

        Ok(())
    }
}

/// Return whether a decorator expression requires parentheses.
fn decorator_needs_parentheses(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
    match tree.get(expression_id) {
        Expression::Parenthesized { .. } => false,
        Expression::Path {
            static_arguments, ..
        } => static_arguments.is_some(),
        Expression::Call { left, .. } => !is_identifier_or_static_member_only(tree, *left),
        Expression::Member {
            left,
            static_arguments,
            ..
        } => static_arguments.is_some() || !is_identifier_or_static_member_only(tree, *left),
        _ => true,
    }
}

/// Return whether an expression is an identifier or static-member-only path.
fn is_identifier_or_static_member_only(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Path {
            static_arguments, ..
        } => static_arguments.is_none(),
        Expression::Member {
            left,
            static_arguments,
            ..
        } => static_arguments.is_none() && is_identifier_or_static_member_only(tree, *left),
        _ => false,
    }
}

/// Format one raw comment or documentation token.
fn format_comment_like_raw_text<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
    is_block_comment: bool,
) -> FormatResult<()> {
    let is_multiline_comment = raw_comment.contains('\n');

    // render one-line comments with trailing whitespace normalized
    if !is_multiline_comment {
        write!(f, [text(raw_comment.trim_end())])?;
        return Ok(());
    }

    // preserve star-aligned block comments in conventional form
    if is_block_comment && block_comment_is_alignable(raw_comment) {
        format_alignable_block_comment(f, raw_comment)?;
        return Ok(());
    }

    format_multiline_comment_raw(f, raw_comment)
}

/// Format one multiline raw comment with explicit line breaks.
fn format_multiline_comment_raw<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
) -> FormatResult<()> {
    let mut lines = raw_comment.lines();
    let Some(first_line) = lines.next() else {
        return Ok(());
    };

    write!(f, [text(first_line.trim_end_matches('\r').trim_end())])?;

    let remaining_lines = lines.collect::<Vec<_>>();
    let common_indent = common_multiline_comment_indent(&remaining_lines);
    for line in remaining_lines {
        let line = line.trim_end_matches('\r');
        let line = if common_indent == 0 {
            line
        } else {
            let mut end_index = 0;
            for byte in line.as_bytes().iter().take(common_indent) {
                if !matches!(*byte, b' ' | b'\t') {
                    break;
                }

                end_index += 1;
            }

            &line[end_index..]
        };
        write!(f, [hard_line_break(), text(line)])?;
    }

    Ok(())
}

/// Return the common leading indentation width for multiline comment lines.
fn common_multiline_comment_indent(lines: &[&str]) -> usize {
    let mut common_indent = usize::MAX;

    for line in lines {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        let line_indent = line
            .as_bytes()
            .iter()
            .take_while(|byte| matches!(**byte, b' ' | b'\t'))
            .count();
        common_indent = common_indent.min(line_indent);
    }

    if common_indent == usize::MAX {
        return 0;
    }

    common_indent
}

/// Format one alignable multiline block comment.
fn format_alignable_block_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
) -> FormatResult<()> {
    let mut lines = raw_comment.lines();
    let Some(first_line) = lines.next() else {
        return Ok(());
    };

    write!(f, [text(first_line.trim_end_matches('\r').trim_end())])?;
    for line in lines {
        let trimmed_line = line.trim_end_matches('\r').trim();
        let normalized_line = if let Some(prefix) = trimmed_line.strip_suffix("*/") {
            let prefix = prefix.trim_end();
            if prefix.is_empty() {
                "*/".to_string()
            } else {
                format!("{prefix} */")
            }
        } else {
            trimmed_line.to_string()
        };
        write!(f, [hard_line_break(), space(), text(&normalized_line)])?;
    }

    Ok(())
}

/// Return whether one multiline block comment is alignable on `*` prefixes.
fn block_comment_is_alignable(raw_comment: &str) -> bool {
    raw_comment
        .lines()
        .skip(1)
        .all(|line| line.trim_start_matches('\r').trim_start().starts_with('*'))
}

impl<'ast> DestackFormatContext<'ast> {
    /// Format the block infix annotations for a node.
    #[inline]
    pub fn block_infix_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
    where
        T: Node + Clone + 'ast,
        NodeTree: NodeTreeImpl<T>,
    {
        format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let items = annotation_render_items_matching(f.context(), node_id, |position| {
                position == AnnotationPosition::BlockInfix
            });
            write_annotation_render_items(f, &items)
        })
    }

    /// Format the line postfix boundary annotations for a node.
    #[inline]
    pub fn line_postfix_boundary_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
    where
        T: Node + Clone + 'ast,
        NodeTree: NodeTreeImpl<T>,
    {
        format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let items = annotation_render_items_matching(f.context(), node_id, |position| {
                position == AnnotationPosition::LinePostfixBoundary
            });
            write_annotation_render_items(f, &items)
        })
    }

    /// Format the line and block prefix annotations for a node.
    #[inline]
    pub fn any_prefix_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
    where
        T: Node + Clone + 'ast,
        NodeTree: NodeTreeImpl<T>,
    {
        format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let items = annotation_render_items_matching(f.context(), node_id, |position| {
                matches!(
                    position,
                    AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
                )
            });
            write_annotation_render_items(f, &items)
        })
    }

    /// Format the line and block postfix annotations for a node.
    #[inline]
    pub fn any_postfix_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
    where
        T: Node + Clone + 'ast,
        NodeTree: NodeTreeImpl<T>,
    {
        format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let items = annotation_render_items_matching(f.context(), node_id, |position| {
                matches!(
                    position,
                    AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                )
            });
            write_annotation_render_items(f, &items)
        })
    }

    /// Format the line and block postfix annotations for a node, excluding line boundary comments.
    #[inline]
    pub fn any_postfix_except_line_postfix_boundary_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
    where
        T: Node + Clone + 'ast,
        NodeTree: NodeTreeImpl<T>,
    {
        format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let items = annotation_render_items_matching(f.context(), node_id, |position| {
                matches!(
                    position,
                    AnnotationPosition::BlockPostfix | AnnotationPosition::LinePostfix
                )
            });
            write_annotation_render_items(f, &items)
        })
    }

    /// Format the line and block infix or postfix annotations for a node.
    #[inline]
    pub fn any_infix_or_postfix_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
    where
        T: Node + Clone + 'ast,
        NodeTree: NodeTreeImpl<T>,
    {
        format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let items = annotation_render_items_matching(f.context(), node_id, |position| {
                matches!(
                    position,
                    AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                )
            });
            write_annotation_render_items(f, &items)
        })
    }

    /// Format the line and block infix or postfix annotations for a node, excluding line boundary comments.
    #[inline]
    pub fn any_infix_or_postfix_except_line_postfix_boundary_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
    where
        T: Node + Clone + 'ast,
        NodeTree: NodeTreeImpl<T>,
    {
        format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let items = annotation_render_items_matching(f.context(), node_id, |position| {
                matches!(
                    position,
                    AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                )
            });
            write_annotation_render_items(f, &items)
        })
    }
}

/// Collect all annotations that match one position filter.
pub(crate) fn annotation_render_items_matching<'ast, T, F>(
    ctx: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    mut include_position: F,
) -> Vec<LocalNodeId<Annotation>>
where
    NodeTree: NodeTreeImpl<T>,
    T: Node,
    F: FnMut(AnnotationPosition) -> bool,
{
    let mut items = Vec::new();
    ctx.visit_annotations(node_id, |annotation_ids| {
        items.reserve(annotation_ids.len());
        for annotation_id in annotation_ids.iter().copied() {
            if include_position(ctx.annotation(annotation_id).position()) {
                items.push(annotation_id);
            }
        }
    });
    items
}

/// Return whether one annotation is one multiline jsdoc block comment.
fn annotation_is_multiline_jsdoc_comment(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = ctx.annotation(annotation_id);
    let annotation_span = ctx.annotation_span(annotation_id);
    let is_multiline = ctx.has_newline(annotation_span);
    if !is_multiline {
        return false;
    }

    matches!(
        annotation,
        Annotation::Doc { node, .. } if ctx.tree.get::<Doc>(node).style == DocStyle::Star
    )
}

/// Return whether two adjacent annotations should be nestled as jsdoc comments.
fn should_nestle_adjacent_jsdoc_comments(
    ctx: &DestackFormatContext<'_>,
    current_annotation_id: LocalNodeId<Annotation>,
    next_annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if !annotation_is_multiline_jsdoc_comment(ctx, current_annotation_id)
        || !annotation_is_multiline_jsdoc_comment(ctx, next_annotation_id)
    {
        return false;
    }

    let current_span = ctx.annotation_span(current_annotation_id);
    let next_span = ctx.annotation_span(next_annotation_id);
    current_span.end == next_span.start
}

/// Return whether one annotation is one own-line comment.
fn annotation_is_own_line_comment(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    match ctx.annotation(annotation_id) {
        Annotation::Comment { .. } | Annotation::Doc { .. } => {
            ctx.annotation_starts_on_own_line(annotation_id)
        }
        Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
    }
}

/// Return whether one annotation is one inline comment.
fn annotation_is_inline_comment(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    match ctx.annotation(annotation_id) {
        Annotation::Comment { .. } | Annotation::Doc { .. } => {
            !ctx.annotation_starts_on_own_line(annotation_id)
        }
        Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
    }
}

/// Return whether there is a blank line between two annotations.
fn annotations_have_blank_line_between(
    ctx: &DestackFormatContext<'_>,
    current_annotation_id: LocalNodeId<Annotation>,
    next_annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let current_span = ctx.annotation_span(current_annotation_id);
    let next_span = ctx.annotation_span(next_annotation_id);
    current_span
        .gap_to(next_span)
        .is_some_and(|between_span| ctx.has_blank_line(between_span))
}

/// Return whether one annotation uses slash comment syntax.
fn annotation_uses_slash_comment_style(
    ctx: &DestackFormatContext<'_>,
    annotation: Annotation,
) -> bool {
    matches!(
        annotation,
        Annotation::Comment { node, .. }
            if ctx.tree.get::<Comment>(node).style == CommentStyle::Slash
    ) || matches!(
        annotation,
        Annotation::Doc { node, .. }
            if ctx.tree.get::<Doc>(node).style == DocStyle::Slash
    )
}

/// Return whether one annotation uses star comment syntax.
fn annotation_uses_star_comment_style(
    ctx: &DestackFormatContext<'_>,
    annotation: Annotation,
) -> bool {
    matches!(
        annotation,
        Annotation::Comment { node, .. }
            if ctx.tree.get::<Comment>(node).style == CommentStyle::Star
    ) || matches!(
        annotation,
        Annotation::Doc { node, .. }
            if ctx.tree.get::<Doc>(node).style == DocStyle::Star
    )
}

/// Emit own-line decorator seam slash comments for postfix positions.
fn write_own_line_decorator_postfix_slash_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    starts_on_own_line: bool,
    next_token_type: Option<TokenType>,
) -> FormatResult<bool> {
    if !starts_on_own_line {
        return Ok(false);
    }

    if next_token_type != Some(TokenType::At) {
        return Ok(false);
    }

    if !matches!(
        position,
        AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
    ) {
        return Ok(false);
    }

    write!(
        f,
        [
            hard_line_break(),
            format_with(
                |f: &mut DestackFormatter<'ast, '_>| annotation.format_node(annotation_id, f)
            ),
            hard_line_break()
        ]
    )?;

    Ok(true)
}

/// Emit slash line-postfix comments through line_postfix to preserve suffix behavior.
pub(crate) fn write_inline_slash_line_postfix_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    is_slash_comment: bool,
    starts_on_own_line: bool,
    next_token_type: Option<TokenType>,
    follows_separator: bool,
) -> FormatResult<bool> {
    if !is_slash_comment {
        return Ok(false);
    }

    let is_tree_closing_tag_head_line_comment = false;
    if is_tree_closing_tag_head_line_comment {
        return Ok(false);
    }

    // own-line comments should keep normal prefix/postfix spacing semantics
    // line_postfix is only for same-line suffix comments, except decorator seams
    if write_own_line_decorator_postfix_slash_comment(
        f,
        annotation,
        position,
        annotation_id,
        starts_on_own_line,
        next_token_type,
    )? {
        return Ok(true);
    }
    if starts_on_own_line {
        return Ok(false);
    }

    let has_virtual_trailing_separator_boundary = next_token_type.is_some_and(|token_type| {
        matches!(
            token_type,
            TokenType::CloseBracket | TokenType::CloseBrace | TokenType::GreaterThan
        )
    });
    let is_supported_line_postfix_position = position == AnnotationPosition::LinePostfix
        || (position == AnnotationPosition::LinePostfixBoundary
            && (follows_separator || has_virtual_trailing_separator_boundary));
    if !is_supported_line_postfix_position {
        return Ok(false);
    }

    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [space()])?;
        annotation.format_node(annotation_id, f)
    });

    write!(f, [line_postfix(&content, 0)])?;

    Ok(true)
}

/// Emit line prefix ignore directives as own line comments.
pub(crate) fn write_line_prefix_ignore_directive_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    is_ignore_directive_line_prefix_comment: bool,
    starts_on_own_line: bool,
) -> FormatResult<bool> {
    if position != AnnotationPosition::LinePrefix {
        return Ok(false);
    }

    if !is_ignore_directive_line_prefix_comment {
        return Ok(false);
    }

    if !starts_on_own_line {
        return Ok(false);
    }

    write!(
        f,
        [
            hard_line_break(),
            format_with(
                |f: &mut DestackFormatter<'ast, '_>| annotation.format_node(annotation_id, f)
            ),
            hard_line_break()
        ]
    )?;

    Ok(true)
}

/// Emit one annotation continuation indent.
fn write_annotation_continuation_indent<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    let indent_width = f.context().options.indent_width as usize;
    let continuation_indent = " ".repeat(indent_width);
    write!(f, [text(&continuation_indent)])?;

    Ok(())
}

/// Emit one postfix ignore directive comment on its own line.
fn write_postfix_ignore_directive_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
    is_ignore_directive_postfix_comment: bool,
) -> FormatResult<bool> {
    if !is_ignore_directive_postfix_comment {
        return Ok(false);
    }

    write!(
        f,
        [
            hard_line_break(),
            format_with(
                |f: &mut DestackFormatter<'ast, '_>| annotation.format_node(annotation_id, f)
            ),
            hard_line_break()
        ]
    )?;

    Ok(true)
}

/// Format a prepared annotation render-item list for one capture mode.
pub(crate) fn write_annotation_render_items<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<Annotation>],
) -> FormatResult<()> {
    if items.is_empty() {
        return Ok(());
    }

    let mut saw_non_blank_annotation = false;
    let mut previous_was_blank_annotation = false;

    for (annotation_index, annotation_id) in items.iter().copied().enumerate() {
        let annotation = f.context().annotation(annotation_id);
        let position = annotation.position();
        let node_type = match annotation {
            Annotation::Blank { .. } => NodeType::Blank,
            Annotation::Doc { .. } => NodeType::Doc,
            Annotation::Comment { .. } => NodeType::Comment,
            Annotation::Decorator { .. } => NodeType::Decorator,
        };
        let next_annotation_id = items.get(annotation_index + 1).copied();
        let is_slash_comment = annotation_uses_slash_comment_style(f.context(), annotation);
        let is_star_comment = annotation_uses_star_comment_style(f.context(), annotation);
        let follows_separator = f
            .context()
            .annotation_previous_non_whitespace_token_type(annotation_id)
            == Some(TokenType::Comma);
        let starts_on_own_line = f.context().annotation_starts_on_own_line(annotation_id);
        let next_token_type = f
            .context()
            .annotation_next_non_whitespace_token_type(annotation_id);
        let next_non_trivia_token_type = f
            .context()
            .annotation_next_non_trivia_token_type(annotation_id);
        let next_token_is_on_same_line = f
            .context()
            .annotation_next_non_whitespace_token(annotation_id)
            .is_some_and(|next_token| {
                let annotation_span = f.context().annotation_span(annotation_id);
                let anchor_offset = annotation_span.end.saturating_sub(1);
                f.context()
                    .file
                    .is_same_line(anchor_offset, next_token.span.start)
            });
        let is_ignore_directive_postfix_comment = is_slash_comment
            && matches!(
                position,
                AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
                    | AnnotationPosition::BlockPostfix
            )
            && (starts_on_own_line || f.context().annotation_has_leading_newline(annotation_id))
            && !matches!(
                next_non_trivia_token_type,
                Some(TokenType::Dot | TokenType::OpenBracket)
            )
            && matches!(
                annotation,
                Annotation::Comment { node, .. }
                    if comment_node_is_ignore_directive(f.context(), node)
            );
        let is_ignore_directive_line_prefix_comment = is_slash_comment
            && position == AnnotationPosition::LinePrefix
            && matches!(
                annotation,
                Annotation::Comment { node, .. }
                    if comment_node_is_any_ignore_directive(f.context(), node)
            );
        let next_annotation_is_inline_comment =
            next_annotation_id.is_some_and(|next_annotation_id| {
                annotation_is_inline_comment(f.context(), next_annotation_id)
            });
        let next_annotation_is_own_line_comment =
            next_annotation_id.is_some_and(|next_annotation_id| {
                annotation_is_own_line_comment(f.context(), next_annotation_id)
            });
        let next_annotation_should_nestle_jsdoc_comment =
            next_annotation_id.is_some_and(|next_annotation_id| {
                should_nestle_adjacent_jsdoc_comments(
                    f.context(),
                    annotation_id,
                    next_annotation_id,
                )
            });
        let has_blank_line_before_next_annotation =
            next_annotation_id.is_some_and(|next_annotation_id| {
                annotations_have_blank_line_between(f.context(), annotation_id, next_annotation_id)
            });
        let is_blank_annotation = matches!(annotation, Annotation::Blank { .. });

        // skip adjacent blank marker duplicates
        if is_blank_annotation && previous_was_blank_annotation {
            continue;
        }

        // render special-path comments and directives
        if write_postfix_ignore_directive_comment(
            f,
            annotation,
            annotation_id,
            is_ignore_directive_postfix_comment,
        )? || write_inline_slash_line_postfix_comment(
            f,
            annotation,
            position,
            annotation_id,
            is_slash_comment,
            starts_on_own_line,
            next_token_type,
            follows_separator,
        )? || write_line_prefix_ignore_directive_comment(
            f,
            annotation,
            position,
            annotation_id,
            is_ignore_directive_line_prefix_comment,
            starts_on_own_line,
        )? {
            previous_was_blank_annotation = false;
            continue;
        }

        if !is_blank_annotation && !saw_non_blank_annotation {
            saw_non_blank_annotation = true;

            if matches!(
                position,
                AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
            ) {
                if starts_on_own_line {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [space()])?;
                }
            } else if matches!(
                position,
                AnnotationPosition::BlockPrefix
                    | AnnotationPosition::BlockInfix
                    | AnnotationPosition::BlockPostfix
            ) && is_star_comment
                && !starts_on_own_line
            {
                write!(f, [space()])?;
            } else if position != AnnotationPosition::LinePrefix || starts_on_own_line {
                write!(f, [hard_line_break()])?;
            }
        }

        let should_skip_dependency_item_separator_blank = node_type == NodeType::DependencyItem
            && matches!(
                position,
                AnnotationPosition::BlockPrefix
                    | AnnotationPosition::BlockPostfix
                    | AnnotationPosition::LinePrefix
            );
        let follows_separator_comment = next_annotation_is_own_line_comment;
        if is_blank_annotation
            && should_skip_dependency_item_separator_blank
            && follows_separator_comment
        {
            previous_was_blank_annotation = true;
            continue;
        }

        if annotation_needs_semicolon_guard_continuation_indent(
            f.context(),
            annotation_id,
            position,
            is_slash_comment,
            starts_on_own_line,
        ) {
            write_annotation_continuation_indent(f)?;
        }

        annotation.format_node(annotation_id, f)?;

        if is_blank_annotation {
            previous_was_blank_annotation = true;
            continue;
        }

        if next_annotation_should_nestle_jsdoc_comment {
        } else if next_annotation_is_own_line_comment && has_blank_line_before_next_annotation {
            write!(f, [empty_line()])?;
        } else if next_annotation_is_inline_comment
            || (next_token_is_on_same_line
                && next_token_type.is_some_and(token_type_is_comment_trivia))
        {
            write!(f, [space()])?;
        } else if matches!(position, AnnotationPosition::LinePostfixBoundary) {
            write!(f, [soft_line_break()])?;
        } else if next_token_type.is_none() || next_token_type == Some(TokenType::End) {
        } else {
            write!(f, [hard_line_break()])?;
        }

        previous_was_blank_annotation = false;
    }

    Ok(())
}
