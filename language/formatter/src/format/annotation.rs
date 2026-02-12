use destack_fir::format::{Format, FormatResult, hard_line_break};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

use crate::directive::is_ignore_directive_comment;
use crate::scan::{
    next_non_whitespace_after_annotation, previous_non_whitespace_before_annotation,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Decorator, Doc, DocStyle,
    Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeType,
};

mod defer;

pub(crate) use defer::{
    call_argument_inline_boundary_prefix_annotations, declaration_body_boundary_prefix_annotations,
    declaration_expression_body_boundary_prefix_annotations, is_lambda_arrow_prefix_annotation,
    parameter_type_separator_prefix_annotations,
};

impl<'ast> DestackFormatContext<'ast> {
    /// Format the block infix annotations for a node.
    #[inline]
    pub fn block_infix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockInfix,
            node_id,
        }
    }

    /// Format the block prefix annotations for a node.
    #[inline]
    pub fn block_prefix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockPrefix,
            node_id,
        }
    }

    /// Format the block postfix annotations for a node.
    #[inline]
    pub fn block_postfix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::BlockPostfix,
            node_id,
        }
    }

    /// Format the line prefix annotations for a node.
    #[inline]
    pub fn line_prefix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::LinePrefix,
            node_id,
        }
    }

    /// Format the line postfix annotations for a node.
    #[inline]
    pub fn line_postfix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::LinePostfix,
            node_id,
        }
    }

    /// Format the line postfix boundary annotations for a node.
    #[inline]
    pub fn line_postfix_boundary_annotations<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::LinePostfixBoundary,
            node_id,
        }
    }

    /// Format the line and block prefix annotations for a node.
    #[inline]
    pub fn any_prefix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::AnyPrefix,
            node_id,
        }
    }

    /// Format the line and block postfix annotations for a node.
    #[inline]
    pub fn any_postfix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::AnyPostfix,
            node_id,
        }
    }

    /// Format the line and block infix or postfix annotations for a node.
    #[inline]
    pub fn any_infix_or_postfix_annotations<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::AnyInfixOrPostfix,
            node_id,
        }
    }
}

/// Return whether a separator punctuation immediately follows an annotation.
fn annotation_precedes_separator<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    matches!(
        next_non_whitespace_after_annotation(context, annotation_id),
        Some(',' | ';' | ')' | ']' | '}' | '>' | '(' | '?' | '.' | ':')
    )
}

/// Return the raw annotation line text including its leading indentation.
fn annotation_line_with_indentation<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<String> {
    let span = context.get_span(annotation_id);
    let head_span = Span::new(span.file, 0, span.start);
    let head_source = context.file.get_span_str(head_span)?;
    let line_start = head_source.rfind('\n').map_or(0, |index| index + 1);

    let line_span = Span::new(span.file, line_start as u32, span.end);
    let line_source = context.file.get_span_str(line_span)?;
    Some(line_source.trim_end().to_string())
}

/// Write an annotation line while preserving explicit leading indentation.
fn write_annotation_line_with_indentation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_line: &str,
) -> FormatResult<()> {
    let mut content_start = raw_line.len();
    for (index, character) in raw_line.char_indices() {
        if character == ' ' || character == '\t' {
            continue;
        }
        content_start = index;
        break;
    }

    let leading = &raw_line[..content_start];
    let content = &raw_line[content_start..];

    write!(f, [hard_line_break()])?;
    for character in leading.chars() {
        if character == '\t' {
            write!(f, [token("\t")])?;
        } else {
            write!(f, [space()])?;
        }
    }
    write!(f, [text(content)])?;
    Ok(())
}

/// Return whether the first non-whitespace token after an annotation starts on the same line.
fn annotation_next_token_is_on_same_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = context.get_span::<Annotation>(annotation_id);
    if span.end >= context.file.len {
        return false;
    }

    let tail_span = Span::new(span.file, span.end, context.file.len);
    let Some(tail_source) = context.file.get_span_str(tail_span) else {
        return false;
    };

    for character in tail_source.chars() {
        if character.is_whitespace() {
            if character == '\n' {
                return false;
            }
            continue;
        }

        return true;
    }

    false
}

/// Return whether an annotation directly follows a colon in source.
fn annotation_follows_colon<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    previous_non_whitespace_before_annotation(context, annotation_id) == Some(':')
}

/// Return whether an annotation directly follows an opening delimiter in source.
fn annotation_follows_opening_delimiter<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    matches!(
        previous_non_whitespace_before_annotation(context, annotation_id),
        Some('(' | '[' | '{' | '<')
    )
}
/// Return whether an annotation directly follows a separator in source.
fn annotation_follows_separator<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    previous_non_whitespace_before_annotation(context, annotation_id) == Some(',')
}

/// Return whether an annotation starts on a line with only leading whitespace.
pub(super) fn annotation_starts_on_own_line<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = context.get_span(annotation_id);
    let head_span = Span::new(span.file, 0, span.start);
    let head_source = context.file.get_span_str(head_span).unwrap_or_default();
    let line_start = head_source.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let line_prefix = &head_source[line_start..];
    line_prefix.trim().is_empty()
}

/// Return whether annotation source begins after at least one newline.
fn annotation_has_leading_newline<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = context.get_span(annotation_id);
    let source = context.get_span_str(span);
    source
        .chars()
        .take_while(|character| character.is_whitespace())
        .any(|character| character == '\n')
}

/// Return whether a comment annotation starts at the first non-whitespace position on its line.
fn comment_annotation_starts_on_own_line<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let Annotation::Comment {
        node: comment_id, ..
    } = context.tree.get::<Annotation>(annotation_id)
    else {
        return false;
    };

    let comment_span = context.get_span::<Comment>(*comment_id);
    if let Some((line_index, _)) = context.file.get_position(comment_span.start)
        && let Some(line_span) = context.file.get_line_span(line_index)
    {
        let prefix_span = Span::new(comment_span.file, line_span.start, comment_span.start);
        context.get_span_str(prefix_span).trim().is_empty()
    } else {
        false
    }
}

/// Return comment style for annotation comments.
fn annotation_comment_style(
    context: &DestackFormatContext<'_>,
    annotation: &Annotation,
) -> Option<CommentStyle> {
    let Annotation::Comment { node, .. } = annotation else {
        return None;
    };

    let comment = context.tree.get::<Comment>(*node);
    Some(comment.style)
}

/// Return whether annotation comment should be treated as own-line.
fn annotation_comment_is_own_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    comment_starts_on_own_line: bool,
) -> bool {
    annotation_starts_on_own_line(context, annotation_id)
        || annotation_has_leading_newline(context, annotation_id)
        || comment_starts_on_own_line
}

/// Return the raw annotation line or a trimmed fallback from annotation span text.
fn annotation_raw_line_or_trimmed_source(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> String {
    annotation_line_with_indentation(context, annotation_id).unwrap_or_else(|| {
        let annotation_span = context.get_span::<Annotation>(annotation_id);
        let annotation_source = context.get_span_str(annotation_span);
        annotation_source.trim_end().to_string()
    })
}

/// Return whether node has an if ancestor and a member ancestor.
fn annotation_ancestor_flags<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> (bool, bool)
where
    NodeTree: NodeTreeImpl<T>,
{
    let mut has_if_ancestor = false;
    let mut has_member_ancestor = false;

    context.any_ancestor(node_id, |ancestor_id, node_type| {
        if node_type == NodeType::Member {
            has_member_ancestor = true;
        }

        if node_type == NodeType::Expression
            && matches!(
                context
                    .tree
                    .get::<Expression>(LocalNodeId::<Expression>::new(ancestor_id)),
                Expression::If { .. }
            )
        {
            has_if_ancestor = true;
        }

        has_if_ancestor && has_member_ancestor
    });

    (has_if_ancestor, has_member_ancestor)
}

/// Return whether node naturally behaves like a member chain root.
fn node_has_member_shape<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(node_id.id);
    match context.tree.get::<Expression>(expression_id) {
        Expression::Member { .. } | Expression::PrivateMember { .. } => true,
        Expression::Path { path, .. } => path.segments.len() > 1,
        _ => false,
    }
}

/// Node-level context shared by all annotation formatting within one node.
#[derive(Debug, Clone, Copy)]
struct AnnotationNodeContext {
    /// Whether any ancestor expression is an if expression.
    has_if_ancestor: bool,
    /// Whether annotation placement should use member-context rules.
    has_member_context: bool,
}

/// Return shared annotation rendering context for a node.
fn annotation_node_context<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> AnnotationNodeContext
where
    NodeTree: NodeTreeImpl<T>,
{
    let node_index = node_id.id;
    let current_node_id = LocalNodeId::<T>::new(node_index);
    let (has_if_ancestor, has_member_ancestor) =
        annotation_ancestor_flags(context, current_node_id);
    let current_node_id = LocalNodeId::<T>::new(node_index);
    let has_member_context = has_member_ancestor || node_has_member_shape(context, current_node_id);

    AnnotationNodeContext {
        has_if_ancestor,
        has_member_context,
    }
}

/// Annotation-level rendering facts computed once and reused across branches.
#[derive(Debug, Clone, Copy)]
struct AnnotationRenderFacts {
    /// Whether annotation is a slash comment.
    is_slash_comment: bool,
    /// Whether annotation is a star comment.
    is_star_comment: bool,
    /// Whether slash comment should be treated as own-line.
    slash_starts_on_own_line: bool,
    /// Whether annotation follows a colon.
    follows_colon: bool,
    /// Whether annotation follows an opening delimiter.
    follows_opening_delimiter: bool,
    /// Whether annotation precedes a separator.
    /// Whether annotation follows a separator.
    follows_separator: bool,
    precedes_separator: bool,
    /// First non-whitespace character after the annotation.
    next_character: Option<char>,
}

/// Return reusable rendering facts for one annotation.
fn annotation_render_facts(
    context: &DestackFormatContext<'_>,
    annotation: &Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> AnnotationRenderFacts {
    let comment_style = annotation_comment_style(context, annotation);
    let is_slash_comment = comment_style == Some(CommentStyle::Slash);
    let is_star_comment = comment_style == Some(CommentStyle::Star);
    let comment_starts_on_own_line = if is_slash_comment {
        comment_annotation_starts_on_own_line(context, annotation_id)
    } else {
        false
    };
    let slash_starts_on_own_line = if is_slash_comment {
        annotation_comment_is_own_line(context, annotation_id, comment_starts_on_own_line)
    } else {
        false
    };

    AnnotationRenderFacts {
        is_slash_comment,
        is_star_comment,
        slash_starts_on_own_line,
        follows_colon: annotation_follows_colon(context, annotation_id),
        follows_separator: annotation_follows_separator(context, annotation_id),
        follows_opening_delimiter: annotation_follows_opening_delimiter(context, annotation_id),
        precedes_separator: annotation_precedes_separator(context, annotation_id),
        next_character: next_non_whitespace_after_annotation(context, annotation_id),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotationCapture {
    BlockInfix,
    BlockPrefix,
    BlockPostfix,
    LinePrefix,
    LinePostfix,
    LinePostfixBoundary,

    AnyPrefix,
    AnyPostfix,
    AnyInfixOrPostfix,
}

/// Annotations for a node.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotations<T: Node> {
    /// The position of the annotations.
    position: AnnotationCapture,
    /// The node ID.
    node_id: LocalNodeId<T>,
}

/// Return whether capture mode includes this annotation position.
fn annotation_capture_includes_position(
    capture: AnnotationCapture,
    position: AnnotationPosition,
) -> bool {
    match position {
        AnnotationPosition::BlockInfix => {
            capture == AnnotationCapture::BlockInfix
                || capture == AnnotationCapture::AnyInfixOrPostfix
        }
        AnnotationPosition::BlockPrefix => {
            capture == AnnotationCapture::BlockPrefix || capture == AnnotationCapture::AnyPrefix
        }
        AnnotationPosition::BlockPostfix => {
            capture == AnnotationCapture::BlockPostfix
                || capture == AnnotationCapture::AnyPostfix
                || capture == AnnotationCapture::AnyInfixOrPostfix
        }
        AnnotationPosition::LinePrefix => {
            capture == AnnotationCapture::LinePrefix || capture == AnnotationCapture::AnyPrefix
        }
        AnnotationPosition::LinePostfix => {
            capture == AnnotationCapture::LinePostfix
                || capture == AnnotationCapture::AnyPostfix
                || capture == AnnotationCapture::AnyInfixOrPostfix
        }
        AnnotationPosition::LinePostfixBoundary => {
            capture == AnnotationCapture::LinePostfixBoundary
                || capture == AnnotationCapture::AnyPostfix
                || capture == AnnotationCapture::AnyInfixOrPostfix
        }
    }
}

impl<'ast, T> Format<DestackFormatContext<'ast>> for Annotations<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
{
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let Some(annotations) = f.context().get_annotations(self.node_id) else {
            return Ok(());
        };
        let node_context = annotation_node_context(f.context(), self.node_id);
        let mut first_node_type: Option<NodeType> = None;
        let mut previous_was_blank_annotation = false;
        for annotation_id in annotations {
            // read annotation
            let annotation = f.context().tree.get::<Annotation>(annotation_id);
            let (node_type, position) = match annotation {
                Annotation::Blank { position, .. } => (NodeType::Blank, *position),
                Annotation::Doc { position, .. } => (NodeType::Doc, *position),
                Annotation::Comment { position, .. } => (NodeType::Comment, *position),
                Annotation::Decorator { position, .. } => (NodeType::Decorator, *position),
            };
            // defer boundary-sensitive annotations to specialized formatters
            if defer::annotation_should_defer(f.context(), self.node_id, annotation_id, position) {
                continue;
            }

            // filter annotation
            let is_included = annotation_capture_includes_position(self.position, position);
            if !is_included {
                continue;
            }

            // collect annotation-specific rendering facts
            let render_facts = annotation_render_facts(f.context(), annotation, annotation_id);
            let is_ignore_directive_postfix_comment = render_facts.is_slash_comment
                && matches!(
                    position,
                    AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockPostfix
                )
                && {
                    let annotation_span = f.context().get_span::<Annotation>(annotation_id);
                    let annotation_source = f.context().get_span_str(annotation_span);
                    let comment_source = if let Annotation::Comment {
                        node: comment_id, ..
                    } = annotation
                    {
                        let comment = f.context().tree.get::<Comment>(*comment_id);
                        f.context().strings.get(comment.string)
                    } else {
                        ""
                    };

                    if render_facts.slash_starts_on_own_line {
                        is_ignore_directive_comment(annotation_source)
                            || is_ignore_directive_comment(comment_source)
                    } else {
                        false
                    }
                };
            let is_inline_decorator_prefix = matches!(annotation, Annotation::Decorator { .. })
                && position == AnnotationPosition::BlockPrefix
                && f.context().options.language_type.is_typescript()
                && annotation_next_token_is_on_same_line(f.context(), annotation_id);
            let is_blank_prefix_annotation = matches!(
                annotation,
                Annotation::Blank {
                    position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                    ..
                }
            );
            let is_blank_annotation = matches!(annotation, Annotation::Blank { .. });
            if is_blank_annotation && previous_was_blank_annotation {
                continue;
            }
            if is_blank_prefix_annotation
                && node_context.has_if_ancestor
                && defer::annotation_followed_by_else_keyword(f.context(), annotation_id)
            {
                continue;
            }
            // keep formatter directives attached to the ignored next node
            if is_ignore_directive_postfix_comment {
                continue;
            }
            let is_inline_block_star_comment = matches!(
                position,
                AnnotationPosition::BlockPrefix | AnnotationPosition::BlockInfix
            ) && render_facts.is_star_comment
                && !annotation_starts_on_own_line(f.context(), annotation_id)
                && !render_facts.follows_colon;
            let is_inline_delimited_block_postfix_star_comment = position
                == AnnotationPosition::BlockPostfix
                && render_facts.is_star_comment
                && render_facts.next_character == Some(',');
            let inline_block_comment_follows_opening_delimiter =
                is_inline_block_star_comment && render_facts.follows_opening_delimiter;

            if render_facts.is_slash_comment
                && matches!(
                    position,
                    AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
                )
            {
                let is_member_chain_boundary = node_context.has_member_context
                    && matches!(render_facts.next_character, Some('.' | '?'));
                let should_preserve_own_line_indentation = render_facts.slash_starts_on_own_line
                    && (is_member_chain_boundary
                        || !render_facts.precedes_separator
                        || matches!(render_facts.next_character, Some(';' | '(')));
                if should_preserve_own_line_indentation {
                    let raw_line =
                        annotation_raw_line_or_trimmed_source(f.context(), annotation_id);
                    if !raw_line.trim().is_empty() {
                        write_annotation_line_with_indentation(f, raw_line.as_str())?;
                        continue;
                    }
                }

                let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(f, [space()])?;
                    if node_context.has_if_ancestor {
                        let annotation_span = f.context().get_span::<Annotation>(annotation_id);
                        let annotation_source = f.context().get_span_str(annotation_span);
                        write!(f, [text(annotation_source.trim())])
                    } else {
                        annotation.format_node(annotation_id, f)
                    }
                });
                write!(f, [line_postfix(&content, 0)])?;
                continue;
            }

            if render_facts.is_slash_comment
                && position == AnnotationPosition::BlockPostfix
                && !node_context.has_member_context
            {
                let should_preserve_own_line_indentation = render_facts.slash_starts_on_own_line
                    && (!render_facts.precedes_separator
                        || matches!(render_facts.next_character, Some(';' | '(')));
                if should_preserve_own_line_indentation {
                    let raw_line =
                        annotation_raw_line_or_trimmed_source(f.context(), annotation_id);
                    if !raw_line.trim().is_empty() {
                        write_annotation_line_with_indentation(f, raw_line.as_str())?;
                        continue;
                    }
                }
            }

            if render_facts.is_slash_comment && position == AnnotationPosition::BlockPrefix {
                let should_preserve_member_chain_prefix_line =
                    node_context.has_member_context && render_facts.slash_starts_on_own_line;
                if should_preserve_member_chain_prefix_line {
                    let annotation_span = f.context().get_span::<Annotation>(annotation_id);
                    let annotation_source = f.context().get_span_str(annotation_span);
                    write!(f, [hard_line_break(), text(annotation_source.trim())])?;
                    write!(f, [hard_line_break()])?;
                    continue;
                }

                let annotation_span = f.context().get_span::<Annotation>(annotation_id);
                let annotation_source = f.context().get_span_str(annotation_span);
                let should_preserve_alignment_marker_indentation =
                    annotation_starts_on_own_line(f.context(), annotation_id)
                        && annotation_source.trim_start().starts_with("//<-");
                if should_preserve_alignment_marker_indentation {
                    let raw_line = annotation_line_with_indentation(f.context(), annotation_id)
                        .unwrap_or_else(|| annotation_source.trim_end().to_string());
                    write_annotation_line_with_indentation(f, raw_line.as_str())?;
                    write!(f, [hard_line_break()])?;
                    continue;
                }
            }

            // keep boundary block comments after member terminators
            let is_member_boundary_comment = if let Annotation::Comment {
                node: comment_id, ..
            } = annotation
            {
                let comment = f.context().tree.get::<Comment>(*comment_id);
                comment.style == CommentStyle::Star
                    && position == AnnotationPosition::LinePostfixBoundary
                    && node_context.has_member_context
            } else {
                false
            };

            if is_member_boundary_comment {
                let content = format_with(|f| {
                    write!(f, [space()])?;
                    annotation.format_node(annotation_id, f)
                });
                write!(f, [line_postfix(&content, 0)])?;
                continue;
            }

            // keep if boundary block comments inline between then and else
            let is_if_boundary_block_comment = if let Annotation::Comment {
                node: comment_id, ..
            } = annotation
            {
                let comment = f.context().tree.get::<Comment>(*comment_id);
                comment.style == CommentStyle::Star
                    && position == AnnotationPosition::LinePostfixBoundary
                    && node_context.has_if_ancestor
                    && !annotation_starts_on_own_line(f.context(), annotation_id)
            } else {
                false
            };

            if is_if_boundary_block_comment {
                if first_node_type.is_none() {
                    first_node_type = Some(node_type);
                    write!(f, [space()])?;
                }

                let annotation_span = f.context().get_span::<Annotation>(annotation_id);
                let annotation_source = f.context().get_span_str(annotation_span);
                write!(f, [text(annotation_source.trim())])?;
                if !render_facts.precedes_separator {
                    write!(f, [space()])?;
                }
                continue;
            }

            // insert space/newline for first annotation in group
            if first_node_type.is_none() {
                first_node_type = Some(node_type);
                let is_block_prefix_after_colon = position == AnnotationPosition::BlockPrefix
                    && render_facts.is_star_comment
                    && render_facts.follows_colon;

                // insert space / newline
                match position {
                    AnnotationPosition::BlockInfix => {
                        if is_inline_block_star_comment {
                            if !inline_block_comment_follows_opening_delimiter {
                                write!(f, [space()])?;
                            }
                        } else {
                            write!(f, [hard_line_break()])?;
                        }
                    }
                    AnnotationPosition::BlockPostfix => {
                        if is_inline_delimited_block_postfix_star_comment {
                            if !render_facts.follows_opening_delimiter {
                                write!(f, [space()])?;
                            }
                        } else {
                            write!(f, [hard_line_break()])?;
                        }
                    }
                    AnnotationPosition::BlockPrefix => {
                        if is_inline_block_star_comment {
                            if !inline_block_comment_follows_opening_delimiter {
                                write!(f, [space()])?;
                            }
                        } else if is_inline_decorator_prefix {
                            write!(f, [space()])?;
                        } else if !is_block_prefix_after_colon {
                            write!(f, [hard_line_break()])?;
                        }
                    }
                    AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary => {
                        // block comments in line postfix still need spacing (slash handled above)
                        write!(f, [space()])?;
                    }
                    AnnotationPosition::LinePrefix => {
                        // no spacing needed for line prefix
                    }
                }
            }

            let is_block_prefix_before_type_grouping_operator = position
                == AnnotationPosition::BlockPrefix
                && T::TYPE == NodeType::Expression
                && matches!(annotation, Annotation::Comment { .. })
                && {
                    let annotation_span = f.context().get_span::<Annotation>(annotation_id);
                    let annotation_source = f.context().get_span_str(annotation_span);
                    let trimmed = annotation_source.trim_start();
                    annotation_source.contains('\n') && !trimmed.starts_with("/**")
                }
                && matches!(render_facts.next_character, Some('|' | '&'));

            // format annotation itself
            annotation.format_node(annotation_id, f)?;

            // insert space / newline
            match position {
                AnnotationPosition::LinePrefix => {
                    write!(f, [space()])?;
                }
                AnnotationPosition::LinePostfix => {
                    if !(render_facts.is_star_comment && render_facts.precedes_separator) {
                        write!(f, [space()])?;
                    }
                }
                AnnotationPosition::BlockInfix => {
                    if is_inline_block_star_comment {
                        if !render_facts.precedes_separator {
                            write!(f, [space()])?;
                        }
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                AnnotationPosition::BlockPostfix => {
                    if is_inline_delimited_block_postfix_star_comment {
                        if !render_facts.precedes_separator {
                            write!(f, [space()])?;
                        }
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                AnnotationPosition::BlockPrefix => {
                    if is_inline_block_star_comment {
                        if !render_facts.precedes_separator {
                            write!(f, [space()])?;
                        }
                    } else if is_inline_decorator_prefix
                        || is_block_prefix_before_type_grouping_operator
                    {
                        write!(f, [space()])?;
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                AnnotationPosition::LinePostfixBoundary => {
                    let should_keep_inline_star_boundary_comment = render_facts.is_star_comment
                        && (render_facts.precedes_separator || render_facts.follows_separator);
                    if !should_keep_inline_star_boundary_comment {
                        write!(f, [soft_line_break()])?;
                    }
                }
            }

            previous_was_blank_annotation = is_blank_annotation;
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Annotation> for Annotation {
    fn format_node(
        &self,
        node_id: LocalNodeId<Annotation>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            Annotation::Blank { node, .. } => {
                // skip blanks at the end of the source
                let container = f
                    .context()
                    .find_ancestor(node_id, |_, node_type| node_type == NodeType::Declaration);
                if let Some((container_id, _)) = container {
                    let container_span = f.context().get_span_by_id(container_id);
                    if container_span.end >= f.context().file.len - 1 {
                        return Ok(());
                    }
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
    fn format_node(
        &self,
        _node_id: LocalNodeId<Doc>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let string = f.context().strings.get(self.string);
        let is_multi_line = string.contains('\n');
        match self.style {
            DocStyle::Star => {
                if is_multi_line {
                    let total_lines = string.lines().count();
                    for (i, line) in string.lines().enumerate() {
                        if i == 0 {
                            write!(f, [token("/**")])?;
                        } else {
                            write!(f, [token(" *")])?;
                        }
                        if !line.is_empty() {
                            write!(f, [space(), text(line)])?;
                        } else if i == 0 {
                            write!(f, [space()])?;
                        }
                        if i != total_lines - 1 {
                            write!(f, [hard_line_break()])?;
                        }
                    }
                    if string.ends_with('\n') {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [token(" */")])?;
                } else {
                    let content = normalize_inline_block_comment_content(string);
                    if content.is_empty() {
                        write!(f, [token("/**/")])?;
                    } else {
                        write!(
                            f,
                            [token("/**"), space(), text(content), space(), token("*/")]
                        )?;
                    }
                }
            }
            DocStyle::Slash => {
                format_line_comment_lines(f, "///", string)?;
            }
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Comment> for Comment {
    fn format_node(
        &self,
        node_id: LocalNodeId<Comment>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let string = f.context().strings.get(self.string);
        let is_multi_line = string.contains('\n');
        match self.style {
            CommentStyle::Star => {
                if is_multi_line {
                    let lines: Vec<&str> = string.lines().collect();
                    let aligns_with_stars = lines
                        .iter()
                        .skip(1)
                        .filter(|line| !line.trim().is_empty())
                        .all(|line| line.trim_start().starts_with('*'));

                    if aligns_with_stars {
                        for (i, line) in lines.iter().enumerate() {
                            if i == 0 {
                                write!(f, [token("/*")])?;
                            } else {
                                write!(f, [token(" *")])?;
                            }
                            if !line.is_empty() {
                                write!(f, [space(), text(line.trim_end_matches('\r'))])?;
                            }
                            if i != lines.len() - 1 {
                                write!(f, [hard_line_break()])?;
                            }
                        }
                        if string.ends_with('\n') {
                            write!(f, [hard_line_break()])?;
                        }
                        write!(f, [token(" */")])?;
                    } else {
                        let source = f.context().get_span_str(f.context().get_span(node_id));
                        let source = source.replace("\r\n", "\n");
                        write!(f, [text(source.trim_end_matches('\n'))])?;
                    }
                } else {
                    let content = normalize_inline_block_comment_content(string);
                    if is_compact_hint_comment(content) {
                        write!(f, [token("/*"), text(content), token("*/")])?;
                    } else if is_all_asterisks_comment(content) {
                        let source = f.context().get_span_str(f.context().get_span(node_id));
                        write!(f, [text(source.trim())])?;
                    } else if content.is_empty() {
                        let source = f.context().get_span_str(f.context().get_span(node_id));
                        if source.contains("/**/") {
                            write!(f, [token("/**/")])?;
                        } else {
                            write!(f, [token("/* */")])?;
                        }
                    } else {
                        write!(
                            f,
                            [token("/*"), space(), text(content), space(), token("*/")]
                        )?;
                    }
                }
            }
            CommentStyle::Slash => {
                format_line_comment_lines(f, "//", string)?;
            }
        }
        Ok(())
    }
}

/// Format slash style comment lines, preserving empty comment lines.
fn format_line_comment_lines<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    prefix: &'static str,
    content: &str,
) -> FormatResult<()> {
    if content.is_empty() {
        write!(f, [token(prefix)])?;
        return Ok(());
    }

    let mut lines = content.split('\n').peekable();
    while let Some(line) = lines.next() {
        if line.is_empty() {
            write!(f, [token(prefix)])?;
        } else {
            let first_character = line.chars().next();
            let should_insert_space = first_character
                .is_some_and(|character| !character.is_ascii_digit() && !line.starts_with("<-"));
            if should_insert_space {
                write!(f, [token(prefix), space(), text(line)])?;
            } else {
                write!(f, [token(prefix), text(line)])?;
            }
        }

        if lines.peek().is_some() {
            write!(f, [hard_line_break()])?;
        }
    }

    Ok(())
}

/// Normalize inline block comment content for stable output.
fn normalize_inline_block_comment_content(content: &str) -> &str {
    let content = content.trim();

    if let Some(stripped_doc) = content.strip_prefix("/**")
        && let Some(inner) = stripped_doc.strip_suffix("*/")
    {
        return inner.trim();
    }

    if let Some(stripped_comment) = content.strip_prefix("/*")
        && let Some(inner) = stripped_comment.strip_suffix("*/")
    {
        return inner.trim();
    }

    content
}

/// Return whether a comment is a compact formatting hint.
fn is_compact_hint_comment(content: &str) -> bool {
    matches!(
        content,
        "#__PURE__" | "@__PURE__" | "#__NO_SIDE_EFFECTS__" | "@__NO_SIDE_EFFECTS__"
    )
}

/// Return whether a block comment body is a run of asterisks.
fn is_all_asterisks_comment(content: &str) -> bool {
    !content.is_empty() && content.chars().all(|character| character == '*')
}

impl<'ast> FormatNode<'ast, Decorator> for Decorator {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Decorator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let tree = f.context().tree;
        let expression_span = f.context().get_span(self.expression);
        let expression_source = f.context().get_span_str(expression_span);
        let expression_contains_inline_comment = f.context().has_comment(expression_span);
        if expression_contains_inline_comment {
            write!(f, [token("@"), text(expression_source.trim())])?;
            return Ok(());
        }

        let needs_parentheses = decorator_needs_parentheses(tree, self.expression);
        write!(f, [token("@")])?;
        if needs_parentheses {
            write!(f, [token("(")])?;
        }
        write!(f, [self.expression])?;
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

#[cfg(test)]
mod tests {
    use crate::{
        DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, TestFormatter,
        assert_format,
    };
    use destack_ast::{
        Annotation, AnnotationPosition, Argument, Declaration, DeclarationDescriptor, Expression,
        LocalNodeId, Member, NodeParentIndex, NodeType, Parameter, Property,
    };

    /// Build a formatter context for annotation routing assertions.
    fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
        DestackFormatContext::new(
            DestackFormatOptions::default(),
            DestackFormatArtifacts {
                file: &formatter.file,
                tree: &formatter.tree,
                tokens: &formatter.tokens,
                side_tokens: &formatter.side_tokens,
                side_span: &formatter.side_span,
                strings: &formatter.strings,
                parents: NodeParentIndex::from_tree(&formatter.tree),
            },
        )
    }

    /// Return whether a marker-tagged annotation is routed through defer rules.
    fn marker_annotation_is_deferred(context: &DestackFormatContext<'_>, marker: &str) -> bool {
        let mut saw_marker = false;

        for raw_node_id in 0..context.tree.next_id() {
            let node_type = context.tree.get_node_type(raw_node_id);

            if node_type == NodeType::Expression {
                let node_id = LocalNodeId::<Expression>::new(raw_node_id);
                if let Some(annotation_ids) = context.get_annotations(node_id) {
                    for annotation_id in annotation_ids {
                        let annotation_span = context.get_span(annotation_id);
                        let annotation_source = context.get_span_str(annotation_span);
                        if !annotation_source.contains(marker) {
                            continue;
                        }

                        saw_marker = true;
                        let position = context.tree.get::<Annotation>(annotation_id).position();
                        if super::defer::annotation_should_defer(
                            context,
                            node_id,
                            annotation_id,
                            position,
                        ) {
                            return true;
                        }
                    }
                }
            }

            if node_type == NodeType::Argument {
                let node_id = LocalNodeId::<Argument>::new(raw_node_id);
                if let Some(annotation_ids) = context.get_annotations(node_id) {
                    for annotation_id in annotation_ids {
                        let annotation_span = context.get_span(annotation_id);
                        let annotation_source = context.get_span_str(annotation_span);
                        if !annotation_source.contains(marker) {
                            continue;
                        }

                        saw_marker = true;
                        let position = context.tree.get::<Annotation>(annotation_id).position();
                        if super::defer::annotation_should_defer(
                            context,
                            node_id,
                            annotation_id,
                            position,
                        ) {
                            return true;
                        }
                    }
                }
            }

            if node_type == NodeType::Parameter {
                let node_id = LocalNodeId::<Parameter>::new(raw_node_id);
                if let Some(annotation_ids) = context.get_annotations(node_id) {
                    for annotation_id in annotation_ids {
                        let annotation_span = context.get_span(annotation_id);
                        let annotation_source = context.get_span_str(annotation_span);
                        if !annotation_source.contains(marker) {
                            continue;
                        }

                        saw_marker = true;
                        let position = context.tree.get::<Annotation>(annotation_id).position();
                        if super::defer::annotation_should_defer(
                            context,
                            node_id,
                            annotation_id,
                            position,
                        ) {
                            return true;
                        }
                    }
                }
            }

            if node_type == NodeType::Declaration {
                let node_id = LocalNodeId::<Declaration>::new(raw_node_id);
                if let Some(annotation_ids) = context.get_annotations(node_id) {
                    for annotation_id in annotation_ids {
                        let annotation_span = context.get_span(annotation_id);
                        let annotation_source = context.get_span_str(annotation_span);
                        if !annotation_source.contains(marker) {
                            continue;
                        }

                        saw_marker = true;
                        let position = context.tree.get::<Annotation>(annotation_id).position();
                        if super::defer::annotation_should_defer(
                            context,
                            node_id,
                            annotation_id,
                            position,
                        ) {
                            return true;
                        }
                    }
                }
            }

            if node_type == NodeType::Member {
                let node_id = LocalNodeId::<Member>::new(raw_node_id);
                if let Some(annotation_ids) = context.get_annotations(node_id) {
                    for annotation_id in annotation_ids {
                        let annotation_span = context.get_span(annotation_id);
                        let annotation_source = context.get_span_str(annotation_span);
                        if !annotation_source.contains(marker) {
                            continue;
                        }

                        saw_marker = true;
                        let position = context.tree.get::<Annotation>(annotation_id).position();
                        if super::defer::annotation_should_defer(
                            context,
                            node_id,
                            annotation_id,
                            position,
                        ) {
                            return true;
                        }
                    }
                }
            }

            if node_type == NodeType::Property {
                let node_id = LocalNodeId::<Property>::new(raw_node_id);
                if let Some(annotation_ids) = context.get_annotations(node_id) {
                    for annotation_id in annotation_ids {
                        let annotation_span = context.get_span(annotation_id);
                        let annotation_source = context.get_span_str(annotation_span);
                        if !annotation_source.contains(marker) {
                            continue;
                        }

                        saw_marker = true;
                        let position = context.tree.get::<Annotation>(annotation_id).position();
                        if super::defer::annotation_should_defer(
                            context,
                            node_id,
                            annotation_id,
                            position,
                        ) {
                            return true;
                        }
                    }
                }
            }
        }

        if !saw_marker {
            panic!("expected marker-tagged annotation in supported annotation parent nodes");
        }

        false
    }

    /// Find a marker-tagged annotation attached to an argument node.
    fn find_marker_annotation_on_argument(
        context: &DestackFormatContext<'_>,
        marker: &str,
    ) -> Option<(LocalNodeId<Argument>, LocalNodeId<Annotation>)> {
        for raw_node_id in 0..context.tree.next_id() {
            if context.tree.get_node_type(raw_node_id) != NodeType::Argument {
                continue;
            }

            let argument_id = LocalNodeId::<Argument>::new(raw_node_id);
            let Some(annotation_ids) = context.get_annotations(argument_id) else {
                continue;
            };
            for annotation_id in annotation_ids {
                let annotation_span = context.get_span(annotation_id);
                let annotation_source = context.get_span_str(annotation_span);
                if annotation_source.contains(marker) {
                    return Some((argument_id, annotation_id));
                }
            }
        }

        None
    }

    /// Find an annotation node by source marker text.
    fn find_annotation_by_marker(
        context: &DestackFormatContext<'_>,
        marker: &str,
    ) -> Option<LocalNodeId<Annotation>> {
        for raw_node_id in 0..context.tree.next_id() {
            if context.tree.get_node_type(raw_node_id) != NodeType::Annotation {
                continue;
            }

            let annotation_id = LocalNodeId::<Annotation>::new(raw_node_id);
            let annotation_span = context.get_span(annotation_id);
            let annotation_source = context.get_span_str(annotation_span);
            if annotation_source.contains(marker) {
                return Some(annotation_id);
            }
        }

        None
    }

    /// Find a statement expression whose inner expression is a ternary.
    fn find_statement_ternary_expression(
        context: &DestackFormatContext<'_>,
    ) -> Option<LocalNodeId<Expression>> {
        for raw_node_id in 0..context.tree.next_id() {
            if context.tree.get_node_type(raw_node_id) != NodeType::Expression {
                continue;
            }

            let expression_id = LocalNodeId::<Expression>::new(raw_node_id);
            let Expression::Statement(inner_expression_id) = context.tree.get(expression_id) else {
                continue;
            };
            if matches!(
                context.tree.get(*inner_expression_id),
                Expression::If {
                    kind: destack_ast::IfKind::Ternary,
                    ..
                }
            ) {
                return Some(expression_id);
            }
        }

        None
    }

    /// Statement ternary boundary comments should route through deferral.
    #[test]
    fn test_annotation_defer_rule_statement_ternary_boundary_prefix() {
        let source = "{
    value
        ?
        /* ternary-boundary */
        on_true
        : on_false;
}";
        let (formatter, _) =
            TestFormatter::parse(source, |p| p.eat_block()).expect("parse ternary boundary source");
        let context = context_from_formatter(&formatter);
        let annotation_id = find_annotation_by_marker(&context, "ternary-boundary")
            .expect("expected marker-tagged ternary annotation");
        let expression_id =
            find_statement_ternary_expression(&context).expect("expected statement ternary node");

        assert!(
            super::defer::should_defer_statement_ternary_boundary_prefix_annotation(
                &context,
                expression_id,
                annotation_id,
                AnnotationPosition::LinePrefix,
            )
        );
    }

    /// Parenthesized boundary comments should route through deferral.
    #[test]
    fn test_annotation_defer_rule_parenthesized_boundary() {
        let source = "{
    const value = (left + right /* paren-boundary */);
}";
        let (formatter, _) = TestFormatter::parse(source, |p| p.eat_block())
            .expect("parse parenthesized boundary source");
        let context = context_from_formatter(&formatter);

        assert!(marker_annotation_is_deferred(&context, "paren-boundary"));
    }

    /// Empty-call boundary comments should route through deferral.
    #[test]
    fn test_annotation_defer_rule_call_boundary() {
        let source = "{
    const value = target(/* call-boundary */);
}";
        let (formatter, _) =
            TestFormatter::parse(source, |p| p.eat_block()).expect("parse call boundary source");
        let context = context_from_formatter(&formatter);

        assert!(marker_annotation_is_deferred(&context, "call-boundary"));
    }

    /// Parameter name-to-type comments should route through deferral.
    #[test]
    fn test_annotation_defer_rule_parameter_type_separator_prefix() {
        let source = "{
    function typed(value /* parameter-type */: number) {}
}";
        let (formatter, _) = TestFormatter::parse(source, |p| p.eat_block())
            .expect("parse parameter type separator source");
        let context = context_from_formatter(&formatter);

        assert!(marker_annotation_is_deferred(&context, "parameter-type"));
    }

    /// Lambda parameter-to-arrow comments should route through deferral.
    #[test]
    fn test_annotation_defer_rule_lambda_arrow_prefix() {
        let source = "{
    const mapper = (value) /* lambda-arrow */ => value;
}";
        let (formatter, _) =
            TestFormatter::parse(source, |p| p.eat_block()).expect("parse lambda arrow source");
        let context = context_from_formatter(&formatter);

        assert!(marker_annotation_is_deferred(&context, "lambda-arrow"));
    }

    /// Call argument boundary prefix comments should route through deferral.
    #[test]
    fn test_annotation_defer_rule_call_argument_inline_boundary_prefix() {
        let source = "{
    target(first, // call-argument-boundary
        second);
}";
        let (formatter, _) = TestFormatter::parse(source, |p| p.eat_block())
            .expect("parse call argument boundary source");
        let context = context_from_formatter(&formatter);
        let (argument_id, annotation_id) =
            find_marker_annotation_on_argument(&context, "call-argument-boundary")
                .expect("expected marker-tagged call argument annotation");

        assert!(
            super::defer::is_call_argument_inline_boundary_prefix_annotation(
                &context,
                argument_id,
                annotation_id,
                AnnotationPosition::LinePrefix,
            )
        );
    }

    /// Declaration header-to-body comments should route through deferral.
    #[test]
    fn test_annotation_defer_rule_declaration_body_boundary_prefix() {
        let source = "{
    class Value /* declaration-body */ {}
}";
        let (formatter, _) = TestFormatter::parse(source, |p| p.eat_block())
            .expect("parse declaration body boundary source");
        let context = context_from_formatter(&formatter);

        assert!(marker_annotation_is_deferred(&context, "declaration-body"));
    }

    /// Method signature boundary comments should route through deferral.
    #[test]
    fn test_annotation_defer_rule_method_body_boundary() {
        let source = "{
    class Value {
        method(): number // method-body-boundary
        {
            return 1;
        }
    }
}";
        let (formatter, _) = TestFormatter::parse(source, |p| p.eat_block())
            .expect("parse method body boundary source");
        let context = context_from_formatter(&formatter);

        assert!(marker_annotation_is_deferred(
            &context,
            "method-body-boundary"
        ));
    }

    /// Block comments should retain all their newlines (including leading and trailing newlines).
    #[test]
    fn test_format_block_comment_retain_newlines() {
        let source = r#"{
    /*
     * Comment 1
     */
    let x;

    /*
     * Comment 2.1
     * Comment 2.2
     * Comment 2.3
     */
    let y;
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Decorators should be preserved in order with other annotations.
    #[test]
    fn test_format_decorators_on_struct() {
        let source = r#"{
    // comment before entity
    @entity
    // comment after entity
    // comment before foo
    @foo(1, 2, 3)
    // comment after foo
    struct Entity {}
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Decorator expressions should not grow extra parentheses across formatting.
    #[test]
    fn test_format_decorator_parentheses_are_stable() {
        let source = r#"{
    @(chain.first().second())
    function chained() {}
}"#;
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Multiple comments around an expression should retain their order.
    #[test]
    fn test_format_multiple_comments_around_expression() {
        let source = "{
    // comment part 1
    // comment part 2
    const A = 1;
    // comment part 3
    // comment part 4
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Multiple comments around an expression should retain their order across successive blocks.
    #[test]
    fn test_format_multiple_comments_around_expression_in_successive_blocks() {
        let source = "{
    // comment part 0
    a: {
        // comment part 1
        // comment part 2
        const A = 1;
        // comment part 3
        // comment part 4
    }
    // comment part 5
    // comment part 6
    b: {
        // comment part 7
        // comment part 8
        const B = 2;
        // comment part 9
        // comment part 10
    }
    // comment part 11
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Inline expression comments should be preserved with proper spacing.
    #[test]
    fn test_format_inline_expression_comment() {
        assert_format!(
            "/* Pre-X comment */const X=/* Pre-A comment */A/* A comment */&&B/* B comment */",
            "/* Pre-X comment */ const X = /* Pre-A comment */ A /* A comment */ && B /* B comment */",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default_with_line_width(200)
        );
    }

    /// Keep multiline block doc comments as block comments.
    #[test]
    fn test_format_multi_line_block_doc_comment_stays_block() {
        assert_format!(
            "{
    /** some multiline
     * doc comment
     * over multiple lines */
    const X = 1 
}",
            "{
    /** some multiline
     * doc comment
     * over multiple lines */
    const X = 1;
}",
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Keep multiline postfix comments as block comments.
    #[test]
    fn test_format_multi_line_block_comment_stays_block() {
        assert_format!(
            "{
    const X = 1 /* some comment
    * over multiple lines yo       */
}",
            "{
    const X = 1;
    /* some comment
    * over multiple lines yo       */
}",
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Excessive whitespace in line comments should be preserved.
    #[test]
    fn test_format_excessive_whitespace_in_line_comment() {
        let source = r"{
    // /// An Identity is globally unique identifier for an Entity.
    // struct Identity {
    //     /// The universally unique identifier of this Entity.
    //     id: Uuid
    // }
    const X = 1;
}";
        assert_format!(
            source,
            source,
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Comments inside function call arguments cause expansion.
    #[test]
    fn test_format_comment_in_call_arguments() {
        assert_format!(
            "foo(/* first */ a, /* second */ b)",
            "foo(/* first */ a, /* second */ b)",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Comments inside array literals cause expansion.
    #[test]
    fn test_format_comment_in_array() {
        assert_format!(
            "[/* first */ 1, /* second */ 2, /* third */ 3]",
            "[
    /* first */ 1,
    /* second */ 2,
    /* third */ 3,
]",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Comments inside object literals cause expansion.
    #[test]
    fn test_format_comment_in_object() {
        assert_format!(
            "{ /* key */ a: 1, /* another */ b: 2 }",
            "{
    /* key */ a: 1,
    /* another */ b: 2,
}",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Format trailing comments on array elements to stay with the comma.
    #[test]
    fn test_format_trailing_comment_array() {
        assert_format!(
            "{
    const arr = [
        1,
        2,
        3, // last element
    ];
}",
            "{
    const arr = [
        1,
        2,
        3, // last element
    ];
}",
            |p| p.eat_block(),
            DestackFormatOptions::default()
        );
    }

    /// Comment inside function body.
    #[test]
    fn test_format_comment_in_function_body() {
        assert_format!(
            "function foo() { /* empty */ }",
            "function foo() {\n    /* empty */\n}",
            |p| p.eat_function(&p.mark(), DeclarationDescriptor::default(), false, false),
            DestackFormatOptions::default()
        );
    }

    /// Empty slash star doc comments should stay stable on arrows.
    #[test]
    fn test_format_empty_doc_comment_on_arrow() {
        assert_format!(
            "() /**/ => 1",
            "() /**/ => 1",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Pure hint comments should keep compact style.
    #[test]
    fn test_format_compact_pure_hint_comment() {
        assert_format!(
            "/*#__PURE__*/factory()",
            "/*#__PURE__*/ factory()",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }

    /// Blank lines between array elements should be preserved.
    #[test]
    fn test_format_blank_in_array() {
        assert_format!(
            "[
    1,

    2,
]",
            "[
    1,

    2,
]",
            |p| p.eat_expression(Default::default()),
            DestackFormatOptions::default()
        );
    }
}
