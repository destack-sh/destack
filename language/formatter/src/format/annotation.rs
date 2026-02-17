use destack_fir::format::{Format, FormatResult, hard_line_break};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

use crate::directive::{is_any_ignore_directive_comment, is_ignore_directive_comment};
use crate::scan::{next_non_whitespace_after_span, previous_non_whitespace_before_annotation};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Blank, Comment, CommentStyle, Declaration, Decorator, Doc, DocStyle,
    Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeType,
};

/// Return the concrete content span for an annotation node.
fn annotation_content_span(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Span {
    context.get_annotation_span(annotation_id)
}

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

    /// Format declaration prefix annotations excluding export-head seam comments.
    #[inline]
    pub fn declaration_prefix_annotations(
        &self,
        node_id: LocalNodeId<Declaration>,
    ) -> Annotations<Declaration> {
        Annotations {
            position: AnnotationCapture::DeclarationPrefix,
            node_id,
        }
    }

    /// Format declaration export-head seam comments.
    #[inline]
    pub fn declaration_export_head_annotations(
        &self,
        node_id: LocalNodeId<Declaration>,
    ) -> Annotations<Declaration> {
        Annotations {
            position: AnnotationCapture::DeclarationExportHead,
            node_id,
        }
    }

    /// Format declaration generic-head seam comments.
    #[inline]
    pub fn declaration_generic_head_annotations(
        &self,
        node_id: LocalNodeId<Declaration>,
    ) -> Annotations<Declaration> {
        Annotations {
            position: AnnotationCapture::DeclarationGenericHead,
            node_id,
        }
    }

    /// Return whether one declaration has any generic-head seam comment annotations.
    pub fn has_declaration_generic_head_annotation(
        &self,
        node_id: LocalNodeId<Declaration>,
    ) -> bool {
        let Some(annotation_ids) = self.get_annotations(node_id) else {
            return false;
        };

        annotation_ids.iter().copied().any(|annotation_id| {
            annotation_is_declaration_generic_head_comment(
                self,
                node_id,
                self.get_annotation(annotation_id),
                annotation_id,
            )
        })
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
    // inspect the concrete annotation content span for stable inline spacing decisions
    let span = annotation_content_span(context, annotation_id);
    matches!(
        next_non_whitespace_after_span(context, span),
        Some(',' | ';' | '(' | ')' | '[' | ']' | '}' | '>' | '?' | '.' | ':' | '=')
    )
}

/// Return whether inline block comments can remain tightly bound to the next separator.
#[inline]
fn inline_block_comment_allows_tight_separator(next_character: Option<char>) -> bool {
    matches!(
        next_character,
        Some(',' | ';' | ')' | ']' | '}' | '>' | '?' | '.' | ':' | '=')
    )
}

/// Return whether the first non-whitespace token after an annotation starts on the same line.
fn annotation_next_token_is_on_same_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = annotation_content_span(context, annotation_id);
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
#[inline]
fn is_opening_delimiter_character(character: char) -> bool {
    matches!(character, '(' | '[' | '{' | '<')
}

/// Return whether an annotation directly follows an opening delimiter in source.
fn annotation_follows_opening_delimiter<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    previous_non_whitespace_before_annotation(context, annotation_id)
        .is_some_and(is_opening_delimiter_character)
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
    let span = annotation_content_span(context, annotation_id);
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
    let span = annotation_content_span(context, annotation_id);
    let source = context.get_span_str(span);
    source
        .chars()
        .take_while(|character| character.is_whitespace())
        .any(|character| character == '\n')
}

/// Return whether annotation source is followed by one blank line before the next token.
fn annotation_has_trailing_blank_line<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = annotation_content_span(context, annotation_id);
    if span.end >= context.file.len {
        return false;
    }

    let tail_span = Span::new(span.file, span.end, context.file.len);
    let Some(tail_source) = context.file.get_span_str(tail_span) else {
        return false;
    };

    let mut newline_count = 0usize;
    for character in tail_source.chars() {
        if character == '\n' {
            newline_count += 1;
            if newline_count >= 2 {
                return true;
            }
            continue;
        }

        if character.is_whitespace() {
            continue;
        }

        break;
    }

    false
}

/// Return whether an annotation is slash-style.
fn annotation_is_slash_style(context: &DestackFormatContext<'_>, annotation: &Annotation) -> bool {
    match annotation {
        Annotation::Comment { node, .. } => {
            let comment = context.tree.get::<Comment>(*node);
            comment.style == CommentStyle::Slash
        }
        Annotation::Doc { node, .. } => {
            let doc = context.tree.get::<Doc>(*node);
            doc.style == DocStyle::Slash
        }
        _ => false,
    }
}

/// Return whether an annotation is star-style.
fn annotation_is_star_style(context: &DestackFormatContext<'_>, annotation: &Annotation) -> bool {
    match annotation {
        Annotation::Comment { node, .. } => {
            let comment = context.tree.get::<Comment>(*node);
            comment.style == CommentStyle::Star
        }
        Annotation::Doc { node, .. } => {
            let doc = context.tree.get::<Doc>(*node);
            doc.style == DocStyle::Star
        }
        _ => false,
    }
}

/// Annotation-level rendering facts computed once and reused across branches.
#[derive(Debug, Clone, Copy)]
struct AnnotationRenderFacts {
    /// Whether annotation is a slash comment.
    is_slash_comment: bool,
    /// Whether annotation is a star comment.
    is_star_comment: bool,
    /// Whether annotation follows a colon.
    follows_colon: bool,
    /// Whether annotation follows an opening delimiter.
    follows_opening_delimiter: bool,
    /// Whether annotation follows a separator.
    follows_separator: bool,
    /// Whether annotation precedes a separator.
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
    let is_slash_comment = annotation_is_slash_style(context, annotation);
    let is_star_comment = annotation_is_star_style(context, annotation);

    AnnotationRenderFacts {
        is_slash_comment,
        is_star_comment,
        follows_colon: annotation_follows_colon(context, annotation_id),
        follows_separator: annotation_follows_separator(context, annotation_id),
        follows_opening_delimiter: annotation_follows_opening_delimiter(context, annotation_id),
        precedes_separator: annotation_precedes_separator(context, annotation_id),
        next_character: next_non_whitespace_after_span(
            context,
            annotation_content_span(context, annotation_id),
        ),
    }
}

/// Return whether decorators may stay inline for this owner node type.
#[inline]
fn decorator_can_stay_inline_for_node_type(node_type: NodeType) -> bool {
    !matches!(
        node_type,
        NodeType::Declaration | NodeType::Member | NodeType::Property | NodeType::MatchCase
    )
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
    DeclarationPrefix,
    DeclarationExportHead,
    DeclarationGenericHead,
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
            capture == AnnotationCapture::BlockPrefix
                || capture == AnnotationCapture::AnyPrefix
                || capture == AnnotationCapture::DeclarationPrefix
                || capture == AnnotationCapture::DeclarationExportHead
                || capture == AnnotationCapture::DeclarationGenericHead
        }
        AnnotationPosition::BlockPostfix => {
            capture == AnnotationCapture::BlockPostfix
                || capture == AnnotationCapture::AnyPostfix
                || capture == AnnotationCapture::AnyInfixOrPostfix
        }
        AnnotationPosition::LinePrefix => {
            capture == AnnotationCapture::LinePrefix
                || capture == AnnotationCapture::AnyPrefix
                || capture == AnnotationCapture::DeclarationPrefix
                || capture == AnnotationCapture::DeclarationExportHead
                || capture == AnnotationCapture::DeclarationGenericHead
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

/// Return whether one declaration carries an export modifier.
fn declaration_has_export_modifier(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let declaration = context.tree.get(declaration_id);
    match declaration {
        Declaration::Global { descriptor, .. }
        | Declaration::Namespace { descriptor, .. }
        | Declaration::Type { descriptor, .. }
        | Declaration::ImportAlias { descriptor, .. }
        | Declaration::Struct { descriptor, .. }
        | Declaration::Class { descriptor, .. }
        | Declaration::Enum { descriptor, .. }
        | Declaration::Interface { descriptor, .. }
        | Declaration::Extension { descriptor, .. }
        | Declaration::Function { descriptor, .. } => descriptor.export.is_some(),
    }
}

/// Return whether one declaration has static parameters.
fn declaration_has_static_parameters(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    match context.tree.get(declaration_id) {
        Declaration::Global { .. }
        | Declaration::ImportAlias { .. }
        | Declaration::Function { .. } => false,
        Declaration::Type {
            static_parameters, ..
        } => static_parameters
            .as_ref()
            .is_some_and(|parameters| !parameters.is_empty()),
        Declaration::Namespace { generics, .. }
        | Declaration::Struct { generics, .. }
        | Declaration::Class { generics, .. }
        | Declaration::Enum { generics, .. }
        | Declaration::Interface { generics, .. }
        | Declaration::Extension { generics, .. } => generics
            .static_parameters
            .as_ref()
            .is_some_and(|parameters| !parameters.is_empty()),
    }
}

/// Return whether one annotation is an export-head seam comment for a declaration.
fn annotation_is_declaration_export_head_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(node_id.id);
    if !declaration_has_export_modifier(context, declaration_id) {
        return false;
    }

    let Annotation::Comment {
        node,
        position: AnnotationPosition::LinePrefix,
    } = annotation
    else {
        return false;
    };
    let comment = context.tree.get::<Comment>(node);
    if comment.style != CommentStyle::Slash {
        return false;
    }

    let declaration_span = context.get_span_by_id(node_id.id);
    let annotation_span = context.get_annotation_span(annotation_id);
    if annotation_span.start <= declaration_span.start {
        return false;
    }

    let declaration_start_line = context
        .file
        .get_position(declaration_span.start)
        .map_or(0, |position| position.0);
    let annotation_start_line = context
        .file
        .get_position(annotation_span.start)
        .map_or(0, |position| position.0);

    annotation_start_line == declaration_start_line
}

/// Return whether one annotation is a generic-head seam comment for a declaration.
fn annotation_is_declaration_generic_head_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(node_id.id);
    if !declaration_has_static_parameters(context, declaration_id) {
        return false;
    }

    let Annotation::Comment {
        node,
        position: AnnotationPosition::LinePrefix,
    } = annotation
    else {
        return false;
    };
    let comment = context.tree.get::<Comment>(node);
    if comment.style != CommentStyle::Slash {
        return false;
    }

    let annotation_span = context.get_annotation_span(annotation_id);
    let declaration_span = context.get_span_by_id(node_id.id);
    if annotation_span.start <= declaration_span.start || annotation_span.end > declaration_span.end
    {
        return false;
    }

    if !matches!(
        previous_non_whitespace_before_annotation(context, annotation_id),
        Some(character) if character.is_ascii_alphanumeric() || character == '_'
    ) {
        return false;
    }

    matches!(
        next_non_whitespace_after_span(context, annotation_span),
        Some('<')
    )
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
        let mut first_node_type: Option<NodeType> = None;
        let mut previous_was_blank_annotation = false;
        for annotation_id in annotations {
            // read annotation
            let annotation = f.context().get_annotation(annotation_id);
            let (node_type, position) = match annotation {
                Annotation::Blank { position, .. } => (NodeType::Blank, position),
                Annotation::Doc { position, .. } => (NodeType::Doc, position),
                Annotation::Comment { position, .. } => (NodeType::Comment, position),
                Annotation::Decorator { position, .. } => (NodeType::Decorator, position),
            };
            // filter annotation
            let is_included = annotation_capture_includes_position(self.position, position);
            if !is_included {
                continue;
            }
            if self.position == AnnotationCapture::DeclarationPrefix
                && annotation_is_declaration_export_head_comment(
                    f.context(),
                    self.node_id,
                    annotation,
                    annotation_id,
                )
            {
                continue;
            }
            if self.position == AnnotationCapture::DeclarationPrefix
                && annotation_is_declaration_generic_head_comment(
                    f.context(),
                    self.node_id,
                    annotation,
                    annotation_id,
                )
            {
                continue;
            }
            if self.position == AnnotationCapture::DeclarationExportHead
                && !annotation_is_declaration_export_head_comment(
                    f.context(),
                    self.node_id,
                    annotation,
                    annotation_id,
                )
            {
                continue;
            }
            if self.position == AnnotationCapture::DeclarationGenericHead
                && !annotation_is_declaration_generic_head_comment(
                    f.context(),
                    self.node_id,
                    annotation,
                    annotation_id,
                )
            {
                continue;
            }

            // collect annotation-specific rendering facts
            let render_facts = annotation_render_facts(f.context(), &annotation, annotation_id);
            let starts_on_own_line = annotation_starts_on_own_line(f.context(), annotation_id);
            let is_ignore_directive_postfix_comment = render_facts.is_slash_comment
                && matches!(
                    position,
                    AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockPostfix
                )
                && {
                    let annotation_span = f.context().get_annotation_span(annotation_id);
                    let annotation_source = f.context().get_span_str(annotation_span);
                    let comment_source = if let Annotation::Comment {
                        node: comment_id, ..
                    } = annotation
                    {
                        let comment = f.context().tree.get::<Comment>(comment_id);
                        f.context().strings.get(comment.string)
                    } else {
                        ""
                    };

                    if annotation_starts_on_own_line(f.context(), annotation_id)
                        || annotation_has_leading_newline(f.context(), annotation_id)
                    {
                        is_ignore_directive_comment(annotation_source)
                            || is_ignore_directive_comment(comment_source)
                    } else {
                        false
                    }
                };
            let decorator_can_stay_inline_for_owner = render_facts.follows_colon
                || render_facts.follows_opening_delimiter
                || render_facts.follows_separator;
            let is_inline_decorator_prefix = matches!(annotation, Annotation::Decorator { .. })
                && position == AnnotationPosition::BlockPrefix
                && !starts_on_own_line
                && annotation_next_token_is_on_same_line(f.context(), annotation_id)
                && decorator_can_stay_inline_for_owner
                && decorator_can_stay_inline_for_node_type(T::TYPE);
            let is_blank_annotation = matches!(annotation, Annotation::Blank { .. });
            if is_blank_annotation && previous_was_blank_annotation {
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
                && annotation_next_token_is_on_same_line(f.context(), annotation_id)
                && !render_facts.follows_colon;
            let is_inline_delimited_block_postfix_star_comment = position
                == AnnotationPosition::BlockPostfix
                && render_facts.is_star_comment
                && render_facts.next_character == Some(',');
            let inline_block_comment_follows_opening_delimiter =
                is_inline_block_star_comment && render_facts.follows_opening_delimiter;
            let is_no_semi_guard_statement_prefix_comment = position
                == AnnotationPosition::BlockPrefix
                && render_facts.is_slash_comment
                && starts_on_own_line
                && render_facts.next_character == Some(';')
                && T::TYPE == NodeType::Expression;

            if render_facts.is_slash_comment
                && position == AnnotationPosition::LinePostfixBoundary
                && starts_on_own_line
            {
                write!(f, [hard_line_break()])?;
                annotation.format_node(annotation_id, f)?;
                write!(f, [hard_line_break()])?;
                continue;
            }

            let can_render_inline_slash_line_postfix = render_facts.is_slash_comment
                && matches!(
                    position,
                    AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
                )
                && !(position == AnnotationPosition::LinePostfixBoundary && starts_on_own_line);
            if can_render_inline_slash_line_postfix {
                let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(f, [space()])?;
                    annotation.format_node(annotation_id, f)
                });
                write!(f, [line_postfix(&content, 0)])?;
                continue;
            }

            if render_facts.is_slash_comment && position == AnnotationPosition::LinePrefix {
                let annotation_span = f.context().get_annotation_span(annotation_id);
                let annotation_source = f.context().get_span_str(annotation_span);

                // keep formatter directives on own lines but let formatter manage indentation
                let comment_source = if let Annotation::Comment {
                    node: comment_id, ..
                } = annotation
                {
                    let comment = f.context().tree.get::<Comment>(comment_id);
                    f.context().strings.get(comment.string)
                } else {
                    ""
                };
                let is_ignore_directive_comment =
                    is_any_ignore_directive_comment(annotation_source)
                        || is_any_ignore_directive_comment(comment_source);
                if is_ignore_directive_comment && !annotation_source.trim().is_empty() {
                    write!(f, [hard_line_break(), text(annotation_source.trim())])?;
                    write!(f, [hard_line_break()])?;
                    continue;
                }
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
                            if !inline_block_comment_follows_opening_delimiter
                                && !starts_on_own_line
                            {
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
                            if !inline_block_comment_follows_opening_delimiter
                                && !starts_on_own_line
                            {
                                write!(f, [space()])?;
                            }
                        } else if is_inline_decorator_prefix {
                            if !render_facts.follows_colon
                                && !render_facts.follows_opening_delimiter
                            {
                                write!(f, [space()])?;
                            }
                        } else if !is_block_prefix_after_colon
                            && !is_no_semi_guard_statement_prefix_comment
                        {
                            write!(f, [hard_line_break()])?;
                        }
                    }
                    AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary => {
                        // keep own-line boundary comments on their own line
                        if position == AnnotationPosition::LinePostfixBoundary && starts_on_own_line
                        {
                            write!(f, [hard_line_break()])?;
                        } else {
                            // block comments in line postfix still need spacing (slash handled above)
                            write!(f, [space()])?;
                        }
                    }
                    AnnotationPosition::LinePrefix => {
                        if self.position == AnnotationCapture::DeclarationGenericHead {
                            write!(f, [space()])?;
                        }
                    }
                }
            }

            let is_block_prefix_before_type_grouping_operator = position
                == AnnotationPosition::BlockPrefix
                && T::TYPE == NodeType::Expression
                && matches!(annotation, Annotation::Comment { .. })
                && {
                    let annotation_span = f.context().get_annotation_span(annotation_id);
                    let annotation_source = f.context().get_span_str(annotation_span);
                    let trimmed = annotation_source.trim_start();
                    !trimmed.starts_with("/**")
                }
                && matches!(render_facts.next_character, Some('|' | '&'));
            // format annotation itself
            if is_no_semi_guard_statement_prefix_comment {
                let annotation_content = format_with(|f| annotation.format_node(annotation_id, f));
                let indented_content =
                    format_with(|f| write!(f, [hard_line_break(), annotation_content]));
                write!(f, [indent(&indented_content)])?;
            } else {
                annotation.format_node(annotation_id, f)?;
            }

            // insert space / newline
            match position {
                AnnotationPosition::LinePrefix => {
                    let next_token_is_on_same_line =
                        annotation_next_token_is_on_same_line(f.context(), annotation_id);
                    if render_facts.is_slash_comment {
                        if next_token_is_on_same_line {
                            write!(f, [space()])?;
                        } else if annotation_has_trailing_blank_line(f.context(), annotation_id) {
                            write!(f, [empty_line()])?;
                        } else {
                            write!(f, [hard_line_break()])?;
                        }
                    } else if next_token_is_on_same_line {
                        write!(f, [space()])?;
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                AnnotationPosition::LinePostfix => {
                    if !(render_facts.is_star_comment && render_facts.precedes_separator) {
                        write!(f, [space()])?;
                    }
                }
                AnnotationPosition::BlockInfix => {
                    if is_inline_block_star_comment {
                        if !render_facts.precedes_separator
                            || !inline_block_comment_allows_tight_separator(
                                render_facts.next_character,
                            )
                        {
                            write!(f, [space()])?;
                        }
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                AnnotationPosition::BlockPostfix => {
                    if is_inline_delimited_block_postfix_star_comment {
                        if !render_facts.precedes_separator
                            || !inline_block_comment_allows_tight_separator(
                                render_facts.next_character,
                            )
                        {
                            write!(f, [space()])?;
                        }
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                AnnotationPosition::BlockPrefix => {
                    if is_inline_block_star_comment {
                        if !render_facts.precedes_separator
                            || !inline_block_comment_allows_tight_separator(
                                render_facts.next_character,
                            )
                        {
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
                    let should_keep_inline_slash_separator_comment = render_facts.is_slash_comment
                        && render_facts.follows_separator
                        && annotation_next_token_is_on_same_line(f.context(), annotation_id);
                    if should_keep_inline_slash_separator_comment {
                        write!(f, [space()])?;
                    } else if render_facts.is_star_comment && !starts_on_own_line {
                        // keep block boundary comments adjacent to list separators
                    } else {
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
                // skip trailing blanks at the end of the source
                let annotation_span = f.context().get_annotation_span(node_id);
                if annotation_span.end >= f.context().file.len.saturating_sub(1) {
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
                        let is_last_line = i == total_lines - 1;
                        let is_last_blank_line = is_last_line && line.trim().is_empty();

                        if is_last_blank_line {
                            // keep a trailing empty line compact as plain closing delimiter
                        } else if i == 0 {
                            write!(f, [token("/**")])?;
                        } else {
                            write!(f, [token(" *")])?;
                        }

                        if !line.is_empty() && !is_last_blank_line {
                            write!(f, [space(), text(line)])?;
                        } else if i == 0 {
                            write!(f, [space()])?;
                        }

                        if !is_last_line {
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
        Annotation, DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions,
        TestFormatter, assert_format,
    };
    use destack_ast::{AnnotationPosition, DeclarationDescriptor, LocalNodeId, NodeParentIndex};
    use destack_source::FileType;

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

    /// Find an annotation node by source marker text.
    fn find_annotation_by_marker(
        context: &DestackFormatContext<'_>,
        marker: &str,
    ) -> Option<LocalNodeId<Annotation>> {
        for (entry_index, entry) in context.formatter_annotation_entries.iter().enumerate() {
            let annotation_id = LocalNodeId::<Annotation>::new(entry_index as u32);
            let annotation_source = context.get_span_str(entry.span);
            if annotation_source.contains(marker) {
                return Some(annotation_id);
            }
        }

        None
    }

    /// Single call argument trailing line comments stay discoverable with stable positions.
    #[test]
    fn test_annotation_single_call_argument_trailing_line_comment_attachment() {
        let source = "{
    someFunction(
        value,
        // trailing-argument-marker
    );
}";
        let (formatter, _) = TestFormatter::parse(source, |p| p.eat_block())
            .expect("parse trailing call argument marker source");
        let context = context_from_formatter(&formatter);

        let annotation_id = find_annotation_by_marker(&context, "trailing-argument-marker")
            .expect("expected trailing marker annotation");
        let position = context.get_annotation(annotation_id).position();

        assert!(matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPostfix
        ));
    }
    /// Type-binary block comments between operator and right type must stay attached and render.
    #[test]
    fn test_type_binary_block_comment_between_operator_and_right_type_renders() {
        let source = "{\n    const value = left as /* between */ Foo;\n}";
        let (formatter, block_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| p.eat_block())
                .expect("parse type-binary block seam source");
        let context = context_from_formatter(&formatter);
        let annotation_id =
            find_annotation_by_marker(&context, "between").expect("expected between annotation");
        let annotation = context.get_annotation(annotation_id);
        assert_eq!(annotation.position(), AnnotationPosition::LinePrefix);

        let formatted = formatter.format(&block_id, DestackFormatOptions::default());
        assert!(
            formatted.contains("as /* between */ Foo"),
            "expected formatted output to preserve operator seam block comment, got:\n{formatted}"
        );
    }

    /// Type-binary block seam comments on expression statements must not be dropped.
    #[test]
    fn test_type_binary_block_comment_between_operator_and_right_type_expression_statement() {
        let source = "{\n    1 as /* between */ Foo;\n    1 satisfies /* sat-between */ Foo;\n}";
        let (formatter, block_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| p.eat_block())
                .expect("parse type-binary block seam statement source");
        let context = context_from_formatter(&formatter);
        let first_annotation_id =
            find_annotation_by_marker(&context, "between").expect("expected between annotation");
        let second_annotation_id = find_annotation_by_marker(&context, "sat-between")
            .expect("expected sat-between annotation");
        assert_eq!(
            context.get_annotation(first_annotation_id).position(),
            AnnotationPosition::LinePrefix
        );
        assert_eq!(
            context.get_annotation(second_annotation_id).position(),
            AnnotationPosition::LinePrefix
        );

        let formatted = formatter.format(&block_id, DestackFormatOptions::default());
        assert!(
            formatted.contains("1 as /* between */ Foo;"),
            "expected formatted output to preserve as seam block comment, got:\n{formatted}"
        );
        assert!(
            formatted.contains("1 satisfies /* sat-between */ Foo;"),
            "expected formatted output to preserve satisfies seam block comment, got:\n{formatted}"
        );
    }

    /// Prefix cast comments before parenthesized values should keep one separating space.
    #[test]
    fn test_prefix_cast_comment_keeps_space_before_parenthesized_value() {
        let source = "{\n    target(/** @type {{id: string}} */ (entry), second);\n}";
        let expected = "{\n    target(/** @type {{id: string}} */ (entry), second);\n}";
        let (formatter, block_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| p.eat_block())
                .expect("parse parenthesized cast argument source");

        let formatted = formatter.format(&block_id, DestackFormatOptions::default());
        assert_eq!(formatted, expected);
    }

    /// Condition boundary comments should detect closing delimiter separators.
    #[test]
    fn test_annotation_render_facts_condition_comment_precedes_separator() {
        let source = "{
    if (true /* separator-marker */ ) {}
}";
        let (formatter, _) =
            TestFormatter::parse(source, |p| p.eat_block()).expect("parse separator marker source");
        let context = context_from_formatter(&formatter);
        let annotation_id = find_annotation_by_marker(&context, "separator-marker")
            .expect("expected marker-tagged separator annotation");
        let annotation = context.get_annotation(annotation_id);
        let facts = super::annotation_render_facts(&context, &annotation, annotation_id);

        assert!(facts.precedes_separator);
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

    /// Decorator prefixed type annotations should stay inline after a colon when simple.
    #[test]
    fn test_format_decorator_type_annotation_stays_inline_after_colon() {
        assert_format!(
            "{\n    const buffer: @addrspace(\"shared\") &Buffer = value;\n}",
            "{\n    const buffer: @addrspace(\"shared\") &Buffer = value;\n}",
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
            "/**/ () => 1",
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

    /// Inline block comments before arrow bodies should keep one separating space and remain idempotent.
    #[test]
    fn test_format_inline_block_comment_before_arrow_body_call_is_idempotent() {
        let source = "{
    const fn = () =>
        /* event, data */doSomething();

    const fn2 = () =>
        /* event, data */doSomething(anything);
}";
        let (first_formatter, first_block_id) = TestFormatter::parse_with_file_type(
            source,
            destack_source::FileType::JavaScript,
            |p| p.eat_block(),
        )
        .expect("parse first arrow comment statement");
        let first = first_formatter.format(&first_block_id, DestackFormatOptions::default());

        let (second_formatter, second_block_id) = TestFormatter::parse_with_file_type(
            first.as_str(),
            destack_source::FileType::JavaScript,
            |p| p.eat_block(),
        )
        .expect("parse second arrow comment statement");
        let second = second_formatter.format(&second_block_id, DestackFormatOptions::default());

        assert_eq!(first, second);
    }
}
