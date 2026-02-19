use destack_fir::format::{Format, FormatResult, hard_line_break};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

use crate::analysis::scan::{
    next_non_whitespace_after_span, previous_non_whitespace_before_annotation,
};
use crate::directive::{is_any_ignore_directive_comment, is_ignore_directive_comment};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Comment, CommentStyle, Declaration, Doc, DocStyle, LocalNodeId, Node,
    NodeTree, NodeTreeImpl, NodeType,
};

/// Return the concrete content span for an annotation node.
fn annotation_content_span(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Span {
    context.annotation_span(annotation_id)
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
        let Some(annotation_ids) = self.annotations(node_id) else {
            return false;
        };

        annotation_ids.iter().copied().any(|annotation_id| {
            annotation_is_declaration_generic_head_comment(
                self,
                node_id,
                self.annotation(annotation_id),
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
    let source = context.span_str(span);
    source
        .chars()
        .take_while(|character| character.is_whitespace())
        .any(|character| character == '\n')
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
pub(super) struct AnnotationRenderFacts {
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
    pub(super) precedes_separator: bool,
    /// First non-whitespace character after the annotation.
    next_character: Option<char>,
}

/// Return reusable rendering facts for one annotation.
pub(super) fn annotation_render_facts(
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

    let declaration_span = context.span_by_id(node_id.id);
    let annotation_span = context.annotation_span(annotation_id);
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

    let annotation_span = context.annotation_span(annotation_id);
    let declaration_span = context.span_by_id(node_id.id);
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
        let Some(annotations) = f.context().annotations(self.node_id) else {
            return Ok(());
        };
        let mut first_node_type: Option<NodeType> = None;
        let mut previous_was_blank_annotation = false;
        for annotation_id in annotations {
            // read annotation
            let annotation = f.context().annotation(annotation_id);
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
                    let annotation_span = f.context().annotation_span(annotation_id);
                    let annotation_source = f.context().span_str(annotation_span);
                    let comment_source = if let Annotation::Comment {
                        node: comment_id, ..
                    } = annotation
                    {
                        f.context().comment_text(comment_id)
                    } else {
                        "".into()
                    };

                    if annotation_starts_on_own_line(f.context(), annotation_id)
                        || annotation_has_leading_newline(f.context(), annotation_id)
                    {
                        is_ignore_directive_comment(annotation_source)
                            || is_ignore_directive_comment(comment_source.as_ref())
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
            if render_facts.is_slash_comment
                && position == AnnotationPosition::LinePostfixBoundary
                && starts_on_own_line
            {
                let comment = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    annotation.format_node(annotation_id, f)
                });
                let indented_comment = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(f, [hard_line_break(), comment])
                });
                write!(f, [indent(&indented_comment), hard_line_break()])?;
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
                let annotation_span = f.context().annotation_span(annotation_id);
                let annotation_source = f.context().span_str(annotation_span);

                // keep formatter directives on own lines but let formatter manage indentation
                let comment_source = if let Annotation::Comment {
                    node: comment_id, ..
                } = annotation
                {
                    f.context().comment_text(comment_id)
                } else {
                    "".into()
                };
                let is_ignore_directive_comment =
                    is_any_ignore_directive_comment(annotation_source)
                        || is_any_ignore_directive_comment(comment_source.as_ref());
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
                        } else if !is_block_prefix_after_colon {
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
                    let annotation_span = f.context().annotation_span(annotation_id);
                    let annotation_source = f.context().span_str(annotation_span);
                    let trimmed = annotation_source.trim_start();
                    !trimmed.starts_with("/**")
                }
                && matches!(render_facts.next_character, Some('|' | '&'));
            // format annotation itself
            annotation.format_node(annotation_id, f)?;

            // insert space / newline
            match position {
                AnnotationPosition::LinePrefix => {
                    let next_token_is_on_same_line =
                        annotation_next_token_is_on_same_line(f.context(), annotation_id);
                    if render_facts.is_slash_comment {
                        if next_token_is_on_same_line {
                            write!(f, [space()])?;
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
                        // but avoid collapsing adjacent block comments
                        if render_facts.next_character == Some('/')
                            && annotation_next_token_is_on_same_line(f.context(), annotation_id)
                        {
                            write!(f, [space()])?;
                        }
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
