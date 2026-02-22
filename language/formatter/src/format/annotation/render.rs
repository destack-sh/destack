use crate::format::analysis::scan::{
    next_non_whitespace_after_span, next_non_whitespace_token_after_span,
    previous_non_whitespace_before_annotation, token_is_keyword,
};
use crate::format::directive::{
    comment_node_is_any_ignore_directive, comment_node_is_ignore_directive,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Comment, CommentStyle, Declaration, Doc, DocStyle, Keyword, LocalNodeId,
    Node, NodeTree, NodeTreeImpl, NodeType,
};
use destack_fir::format::{Format, FormatResult, hard_line_break};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;

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
    DeclarationBodyHead,
}

/// Annotations for a node.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotations<T: Node> {
    /// The position of the annotations.
    pub(crate) position: AnnotationCapture,
    /// The node ID.
    pub(crate) node_id: LocalNodeId<T>,
}

/// Mutable emit state for one annotation group render pass.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct AnnotationRenderState {
    /// The first annotation node type in this pass.
    pub(crate) first_node_type: Option<NodeType>,
    /// Whether the previously emitted annotation was blank.
    pub(crate) previous_was_blank_annotation: bool,
}

/// Return the concrete content span for an annotation node.
pub(crate) fn annotation_content_span(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Span {
    context.annotation_span(annotation_id)
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
pub(crate) fn inline_block_comment_allows_tight_separator(next_character: Option<char>) -> bool {
    matches!(
        next_character,
        Some(',' | ';' | ')' | ']' | '}' | '>' | '?' | '.' | ':' | '=')
    )
}

/// Return whether the first non-whitespace token after an annotation starts on the same line.
pub(crate) fn annotation_next_token_is_on_same_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = annotation_content_span(context, annotation_id);
    let Some(next_token) = next_non_whitespace_token_after_span(context, span) else {
        return false;
    };

    let anchor_offset = span.end.saturating_sub(1);
    context
        .file
        .is_same_line(anchor_offset, next_token.span.start)
}

/// Return whether one annotation is followed by an `else` keyword token.
pub(crate) fn annotation_next_token_is_else_keyword(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = annotation_content_span(context, annotation_id);
    let Some(next_token) = next_non_whitespace_token_after_span(context, span) else {
        return false;
    };

    token_is_keyword(context, next_token, Keyword::Else)
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
pub(crate) fn annotation_starts_on_own_line<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    context.span_starts_on_own_line(annotation_content_span(context, annotation_id))
}

/// Return whether annotation source begins after at least one newline.
pub(crate) fn annotation_has_leading_newline<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = annotation_content_span(context, annotation_id);
    context.has_newline(span)
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
pub(crate) struct AnnotationRenderFacts {
    /// Whether annotation is a slash comment.
    pub(crate) is_slash_comment: bool,
    /// Whether annotation is a star comment.
    pub(crate) is_star_comment: bool,
    /// Whether annotation follows a colon.
    pub(crate) follows_colon: bool,
    /// Whether annotation follows an opening delimiter.
    pub(crate) follows_opening_delimiter: bool,
    /// Whether annotation follows a separator.
    pub(crate) follows_separator: bool,
    /// Whether annotation precedes a separator.
    pub(crate) precedes_separator: bool,
    /// First non-whitespace character after the annotation.
    pub(crate) next_character: Option<char>,
}

/// Return reusable rendering facts for one annotation.
pub(crate) fn annotation_render_facts(
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

/// Return whether decorators may stay inline via source-seam facts for this owner node type.
#[inline]
pub(crate) fn decorator_can_stay_inline_for_comment_seam(node_type: NodeType) -> bool {
    !matches!(
        node_type,
        NodeType::Declaration
            | NodeType::Property
            | NodeType::Member
            | NodeType::EnumField
            | NodeType::MatchCase
    )
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

/// Return whether one annotation is an export-head seam comment for a declaration.
pub(crate) fn annotation_is_declaration_export_head_comment<T: Node>(
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
pub(crate) fn annotation_is_declaration_generic_head_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Declaration {
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
    matches!(
        next_non_whitespace_after_span(context, annotation_span),
        Some('<')
    )
}

/// Return whether one annotation is a declaration-body seam comment before `{`.
pub(crate) fn annotation_is_declaration_body_head_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Declaration {
        return false;
    }

    let Annotation::Comment { position, .. } = annotation else {
        return false;
    };
    if !matches!(
        position,
        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
    ) {
        return false;
    }

    let annotation_span = context.annotation_span(annotation_id);
    matches!(
        next_non_whitespace_after_span(context, annotation_span),
        Some('{')
    )
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

    /// Format declaration body-head seam comments before `{`.
    #[inline]
    pub fn declaration_body_head_annotations(
        &self,
        node_id: LocalNodeId<Declaration>,
    ) -> Annotations<Declaration> {
        Annotations {
            position: AnnotationCapture::DeclarationBodyHead,
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

    /// Return whether one declaration has any body-head seam comment annotations.
    pub fn has_declaration_body_head_annotation(&self, node_id: LocalNodeId<Declaration>) -> bool {
        let Some(annotation_ids) = self.annotations(node_id) else {
            return false;
        };

        annotation_ids.iter().copied().any(|annotation_id| {
            annotation_is_declaration_body_head_comment(
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

/// Return whether capture mode includes this annotation position.
pub(crate) fn annotation_capture_includes_position(
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
                || capture == AnnotationCapture::DeclarationBodyHead
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
                || capture == AnnotationCapture::DeclarationBodyHead
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

/// Return whether capture mode keeps this declaration annotation in the active stream.
pub(crate) fn annotation_is_included_for_capture<T: Node>(
    context: &DestackFormatContext<'_>,
    capture: AnnotationCapture,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if capture == AnnotationCapture::DeclarationPrefix
        && annotation_is_declaration_export_head_comment(
            context,
            LocalNodeId::<T>::new(node_id.id),
            annotation,
            annotation_id,
        )
    {
        return false;
    }

    if capture == AnnotationCapture::DeclarationPrefix
        && annotation_is_declaration_generic_head_comment(
            context,
            LocalNodeId::<T>::new(node_id.id),
            annotation,
            annotation_id,
        )
    {
        return false;
    }

    if capture == AnnotationCapture::DeclarationPrefix
        && annotation_is_declaration_body_head_comment(
            context,
            LocalNodeId::<T>::new(node_id.id),
            annotation,
            annotation_id,
        )
    {
        return false;
    }

    if capture == AnnotationCapture::DeclarationExportHead
        && !annotation_is_declaration_export_head_comment(
            context,
            LocalNodeId::<T>::new(node_id.id),
            annotation,
            annotation_id,
        )
    {
        return false;
    }

    if capture == AnnotationCapture::DeclarationBodyHead
        && !annotation_is_declaration_body_head_comment(
            context,
            LocalNodeId::<T>::new(node_id.id),
            annotation,
            annotation_id,
        )
    {
        return false;
    }

    if capture == AnnotationCapture::DeclarationGenericHead
        && !annotation_is_declaration_generic_head_comment(
            context,
            LocalNodeId::<T>::new(node_id.id),
            annotation,
            annotation_id,
        )
    {
        return false;
    }

    true
}

/// One annotation item eligible for one capture pass.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AnnotationRenderItem {
    /// The concrete annotation id in source order.
    pub(crate) annotation_id: LocalNodeId<Annotation>,
    /// The annotation node type.
    pub(crate) node_type: NodeType,
    /// The concrete capture position.
    pub(crate) position: AnnotationPosition,
}

/// Collect all annotations that belong to one capture pass.
pub(crate) fn collect_annotation_render_items<'ast, T: Node>(
    context: &DestackFormatContext<'ast>,
    capture: AnnotationCapture,
    node_id: LocalNodeId<T>,
) -> Vec<AnnotationRenderItem>
where
    NodeTree: NodeTreeImpl<T>,
{
    let node_raw_id = node_id.id;
    let Some(annotation_ids) = context.annotations(node_id) else {
        return Vec::new();
    };

    let mut items = Vec::with_capacity(annotation_ids.len());

    for annotation_id in annotation_ids.iter().copied() {
        let annotation = context.annotation(annotation_id);
        let (node_type, position) = annotation_node_type_and_position(annotation);

        if !annotation_capture_includes_position(capture, position) {
            continue;
        }

        let node_id_for_capture = LocalNodeId::<T>::new(node_raw_id);
        if !annotation_is_included_for_capture(
            context,
            capture,
            node_id_for_capture,
            annotation,
            annotation_id,
        ) {
            continue;
        }

        items.push(AnnotationRenderItem {
            annotation_id,
            node_type,
            position,
        });
    }

    items
}

/// Extract stable node and position facts for one annotation.
fn annotation_node_type_and_position(annotation: Annotation) -> (NodeType, AnnotationPosition) {
    match annotation {
        Annotation::Blank { position, .. } => (NodeType::Blank, position),
        Annotation::Doc { position, .. } => (NodeType::Doc, position),
        Annotation::Comment { position, .. } => (NodeType::Comment, position),
        Annotation::Decorator { position, .. } => (NodeType::Decorator, position),
    }
}

/// Reusable flow facts for one annotation item.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AnnotationFlowFacts {
    /// Precomputed rendering facts for one annotation.
    pub(crate) render_facts: AnnotationRenderFacts,
    /// Whether this annotation starts on its own line.
    pub(crate) starts_on_own_line: bool,
    /// Whether the next captured annotation is an inline star comment.
    pub(crate) next_annotation_is_inline_star_comment: bool,
    /// Whether the next captured annotation is an own line comment.
    pub(crate) next_annotation_is_own_line_comment: bool,
    /// Whether this is a directive postfix comment that should be skipped.
    pub(crate) is_ignore_directive_postfix_comment: bool,
    /// Whether this is a directive line prefix comment.
    pub(crate) is_ignore_directive_line_prefix_comment: bool,
    /// Whether this annotation is an inline block star comment.
    pub(crate) is_inline_block_star_comment: bool,
    /// Whether this is a block postfix star comment before a separator.
    pub(crate) is_inline_delimited_block_postfix_star_comment: bool,
    /// Whether an inline block comment follows an opening delimiter.
    pub(crate) inline_block_comment_follows_opening_delimiter: bool,
    /// Whether a block prefix decorator can stay inline.
    pub(crate) is_inline_decorator_prefix: bool,
    /// Whether the next token after the annotation is on the same line.
    pub(crate) next_token_is_on_same_line: bool,
    /// Whether the next token after the annotation is `else`.
    pub(crate) next_token_is_else_keyword: bool,
    /// Whether this annotation is a block prefix before a type grouping operator.
    pub(crate) is_block_prefix_before_type_grouping_operator: bool,
}

/// Build flow facts for one annotation item.
pub(crate) fn build_annotation_flow_facts<'ast, T: Node>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
    item: AnnotationRenderItem,
    annotation: Annotation,
) -> AnnotationFlowFacts
where
    NodeTree: NodeTreeImpl<T>,
{
    let render_facts = annotation_render_facts(context, &annotation, item.annotation_id);
    let starts_on_own_line = annotation_starts_on_own_line(context, item.annotation_id);
    let next_annotation_is_inline_star_comment =
        next_item_is_inline_star_comment(context, items, annotation_index);
    let next_annotation_is_own_line_comment =
        next_item_is_own_line_comment(context, items, annotation_index);
    let previous_annotation_is_own_line_slash_comment =
        previous_item_is_own_line_slash_comment(context, items, annotation_index);
    let is_ignore_directive_postfix_comment = annotation_is_ignore_directive_postfix_comment(
        context,
        annotation,
        item.position,
        item.annotation_id,
        render_facts,
        starts_on_own_line,
    );
    let is_ignore_directive_line_prefix_comment =
        annotation_is_ignore_directive_line_prefix_comment(
            context,
            annotation,
            item.position,
            render_facts,
        );

    let next_token_is_on_same_line =
        annotation_next_token_is_on_same_line(context, item.annotation_id);
    let next_token_is_else_keyword =
        annotation_next_token_is_else_keyword(context, item.annotation_id);

    let is_inline_block_star_comment = matches!(
        item.position,
        AnnotationPosition::BlockPrefix | AnnotationPosition::BlockInfix
    ) && render_facts.is_star_comment
        && next_token_is_on_same_line
        && !render_facts.follows_colon;
    let is_inline_delimited_block_postfix_star_comment = item.position
        == AnnotationPosition::BlockPostfix
        && render_facts.is_star_comment
        && render_facts.next_character == Some(',');
    let inline_block_comment_follows_opening_delimiter =
        is_inline_block_star_comment && render_facts.follows_opening_delimiter;
    let is_inline_decorator_prefix = annotation_is_inline_decorator_prefix::<T>(
        context,
        node_id,
        annotation,
        item.position,
        item.annotation_id,
        render_facts,
        previous_annotation_is_own_line_slash_comment,
    );
    let is_block_prefix_before_type_grouping_operator = item.position
        == AnnotationPosition::BlockPrefix
        && T::TYPE == NodeType::Expression
        && matches!(annotation, Annotation::Comment { .. })
        && matches!(render_facts.next_character, Some('|' | '&'));

    AnnotationFlowFacts {
        render_facts,
        starts_on_own_line,
        next_annotation_is_inline_star_comment,
        next_annotation_is_own_line_comment,
        is_ignore_directive_postfix_comment,
        is_ignore_directive_line_prefix_comment,
        is_inline_block_star_comment,
        is_inline_delimited_block_postfix_star_comment,
        inline_block_comment_follows_opening_delimiter,
        is_inline_decorator_prefix,
        next_token_is_on_same_line,
        next_token_is_else_keyword,
        is_block_prefix_before_type_grouping_operator,
    }
}

/// Return whether the next item is an inline star comment.
fn next_item_is_inline_star_comment(
    context: &DestackFormatContext<'_>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
) -> bool {
    items
        .get(annotation_index + 1)
        .copied()
        .is_some_and(|next_item| {
            annotation_is_comment_style(context, next_item.annotation_id, CommentStyle::Star)
                && !annotation_starts_on_own_line(context, next_item.annotation_id)
        })
}

/// Return whether the next item is an own line comment.
fn next_item_is_own_line_comment(
    context: &DestackFormatContext<'_>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
) -> bool {
    items
        .get(annotation_index + 1)
        .copied()
        .is_some_and(|next_item| {
            matches!(
                context.annotation(next_item.annotation_id),
                Annotation::Comment { .. }
            ) && annotation_starts_on_own_line(context, next_item.annotation_id)
        })
}

/// Return whether the previous item is an own line slash comment.
fn previous_item_is_own_line_slash_comment(
    context: &DestackFormatContext<'_>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
) -> bool {
    annotation_index
        .checked_sub(1)
        .and_then(|previous_index| items.get(previous_index).copied())
        .is_some_and(|previous_item| {
            annotation_is_comment_style(context, previous_item.annotation_id, CommentStyle::Slash)
                && annotation_starts_on_own_line(context, previous_item.annotation_id)
        })
}

/// Return whether one annotation is a comment in one concrete style.
fn annotation_is_comment_style(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    comment_style: CommentStyle,
) -> bool {
    match context.annotation(annotation_id) {
        Annotation::Comment { node, .. } => {
            let comment = context.tree.get::<Comment>(node);
            comment.style == comment_style
        }
        _ => false,
    }
}

/// Return whether one annotation is a directive postfix slash comment that should be skipped.
fn annotation_is_ignore_directive_postfix_comment(
    context: &DestackFormatContext<'_>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    render_facts: AnnotationRenderFacts,
    starts_on_own_line: bool,
) -> bool {
    if !render_facts.is_slash_comment {
        return false;
    }

    if !matches!(
        position,
        AnnotationPosition::LinePostfix
            | AnnotationPosition::LinePostfixBoundary
            | AnnotationPosition::BlockPostfix
    ) {
        return false;
    }

    if !starts_on_own_line && !annotation_has_leading_newline(context, annotation_id) {
        return false;
    }

    match annotation {
        Annotation::Comment { node, .. } => comment_node_is_ignore_directive(context, node),
        _ => false,
    }
}

/// Return whether one annotation is a directive line prefix slash comment.
fn annotation_is_ignore_directive_line_prefix_comment(
    context: &DestackFormatContext<'_>,
    annotation: Annotation,
    position: AnnotationPosition,
    render_facts: AnnotationRenderFacts,
) -> bool {
    if !render_facts.is_slash_comment || position != AnnotationPosition::LinePrefix {
        return false;
    }

    match annotation {
        Annotation::Comment { node, .. } => comment_node_is_any_ignore_directive(context, node),
        _ => false,
    }
}

/// Return whether one decorator prefix annotation can stay inline.
fn annotation_is_inline_decorator_prefix<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    render_facts: AnnotationRenderFacts,
    previous_annotation_is_own_line_slash_comment: bool,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if !matches!(annotation, Annotation::Decorator { .. }) {
        return false;
    }

    if position != AnnotationPosition::BlockPrefix {
        return false;
    }

    let decorator_can_stay_inline_for_seam = (render_facts.follows_colon
        || render_facts.follows_opening_delimiter
        || render_facts.follows_separator)
        && decorator_can_stay_inline_for_comment_seam(T::TYPE);
    if decorator_can_stay_inline_for_seam {
        return true;
    }

    let decorator_can_stay_inline_for_same_line_owner = {
        let annotation_span = annotation_content_span(context, annotation_id);
        let node_span = context.span(node_id);
        annotation_span.file == node_span.file
            && annotation_span.end > annotation_span.start
            && context
                .file
                .is_same_line(annotation_span.end.saturating_sub(1), node_span.start)
            && matches!(T::TYPE, NodeType::Property | NodeType::Member)
    };

    decorator_can_stay_inline_for_same_line_owner && previous_annotation_is_own_line_slash_comment
}

/// Emit special indentation for own line line-postfix-boundary slash comments.
pub(crate) fn write_boundary_line_postfix_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    flow_facts: AnnotationFlowFacts,
) -> FormatResult<bool> {
    if !(flow_facts.render_facts.is_slash_comment
        && position == AnnotationPosition::LinePostfixBoundary
        && flow_facts.starts_on_own_line)
    {
        return Ok(false);
    }

    let separator_starts_expression = matches!(
        flow_facts.render_facts.next_character,
        Some('(' | '[' | '{' | '<')
    );
    let separator_requires_boundary_continuation = matches!(
        flow_facts.render_facts.next_character,
        Some(',' | ';' | ':' | '=' | '?' | '.')
    );

    // separator leading boundary markers keep one continuation level
    if separator_requires_boundary_continuation {
        write!(
            f,
            [
                indent(&format_args![
                    hard_line_break(),
                    format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        annotation.format_node(annotation_id, f)
                    })
                ]),
                hard_line_break()
            ]
        )?;
        return Ok(true);
    }

    // expression leading boundary comments stay flush with current indent
    if separator_starts_expression {
        write!(
            f,
            [
                hard_line_break(),
                format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    annotation.format_node(annotation_id, f)
                }),
                hard_line_break()
            ]
        )?;
        return Ok(true);
    }

    Ok(false)
}

/// Emit slash postfix comments through line_postfix to preserve suffix behavior.
pub(crate) fn write_inline_slash_line_postfix_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    flow_facts: AnnotationFlowFacts,
) -> FormatResult<bool> {
    if !flow_facts.render_facts.is_slash_comment {
        return Ok(false);
    }

    if !matches!(
        position,
        AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
    ) {
        return Ok(false);
    }

    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if position == AnnotationPosition::LinePostfixBoundary && flow_facts.starts_on_own_line {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }

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
    flow_facts: AnnotationFlowFacts,
) -> FormatResult<bool> {
    if position != AnnotationPosition::LinePrefix {
        return Ok(false);
    }

    if !flow_facts.is_ignore_directive_line_prefix_comment {
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

/// Emit spacing before the first annotation in one render group.
pub(crate) fn write_first_annotation_spacing<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    capture: AnnotationCapture,
    state: &mut AnnotationRenderState,
    node_type: NodeType,
    position: AnnotationPosition,
    is_blank_annotation: bool,
    flow_facts: AnnotationFlowFacts,
) -> FormatResult<()> {
    if state.first_node_type.is_some() {
        return Ok(());
    }

    state.first_node_type = Some(node_type);

    let is_block_prefix_after_colon = position == AnnotationPosition::BlockPrefix
        && flow_facts.render_facts.is_star_comment
        && flow_facts.render_facts.follows_colon;

    if is_blank_annotation {
        return Ok(());
    }

    match position {
        AnnotationPosition::BlockInfix => {
            if flow_facts.is_inline_block_star_comment {
                if !flow_facts.inline_block_comment_follows_opening_delimiter
                    && !flow_facts.starts_on_own_line
                {
                    write!(f, [space()])?;
                }
            } else {
                write!(f, [hard_line_break()])?;
            }
        }
        AnnotationPosition::BlockPostfix => {
            if flow_facts.is_inline_delimited_block_postfix_star_comment {
                if !flow_facts.render_facts.follows_opening_delimiter {
                    write!(f, [space()])?;
                }
            } else {
                write!(f, [hard_line_break()])?;
            }
        }
        AnnotationPosition::BlockPrefix => {
            if flow_facts.is_inline_block_star_comment {
                if !flow_facts.inline_block_comment_follows_opening_delimiter
                    && !flow_facts.starts_on_own_line
                {
                    write!(f, [space()])?;
                }
            } else if flow_facts.is_inline_decorator_prefix {
                if !flow_facts.starts_on_own_line
                    && !flow_facts.render_facts.follows_colon
                    && !flow_facts.render_facts.follows_opening_delimiter
                {
                    write!(f, [space()])?;
                }
            } else if !is_block_prefix_after_colon {
                write!(f, [hard_line_break()])?;
            }
        }
        AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary => {
            if position == AnnotationPosition::LinePostfixBoundary && flow_facts.starts_on_own_line
            {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [space()])?;
            }
        }
        AnnotationPosition::LinePrefix => {
            if capture == AnnotationCapture::DeclarationGenericHead {
                write!(f, [space()])?;
            } else if capture == AnnotationCapture::DeclarationBodyHead {
                if flow_facts.starts_on_own_line {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [space()])?;
                }
            }
        }
    }

    Ok(())
}

/// Return whether one blank annotation should be skipped before an own line comment.
pub(crate) fn should_skip_blank_annotation_before_own_line_comment(
    position: AnnotationPosition,
    is_blank_annotation: bool,
    flow_facts: AnnotationFlowFacts,
) -> bool {
    is_blank_annotation
        && position == AnnotationPosition::BlockPrefix
        && flow_facts.render_facts.follows_separator
        && flow_facts.next_annotation_is_own_line_comment
}

/// Emit spacing after one annotation.
pub(crate) fn write_annotation_trailing_spacing<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    position: AnnotationPosition,
    flow_facts: AnnotationFlowFacts,
) -> FormatResult<()> {
    match position {
        AnnotationPosition::LinePrefix => {
            if flow_facts.render_facts.is_slash_comment {
                if flow_facts.next_token_is_on_same_line {
                    write!(f, [space()])?;
                } else {
                    write!(f, [hard_line_break()])?;
                }
            } else if flow_facts.render_facts.is_star_comment
                && flow_facts.next_token_is_else_keyword
            {
                write!(f, [space()])?;
            } else if flow_facts.next_token_is_on_same_line {
                write!(f, [space()])?;
            } else {
                write!(f, [hard_line_break()])?;
            }
        }
        AnnotationPosition::LinePostfix => {
            if !(flow_facts.render_facts.is_star_comment
                && flow_facts.render_facts.precedes_separator)
            {
                write!(f, [space()])?;
            }
        }
        AnnotationPosition::BlockInfix => {
            let should_keep_space_before_adjacent_block_comment =
                should_keep_space_before_adjacent_block_comment(flow_facts);
            if flow_facts.is_inline_block_star_comment {
                if !flow_facts.render_facts.precedes_separator
                    || !self::inline_block_comment_allows_tight_separator(
                        flow_facts.render_facts.next_character,
                    )
                    || should_keep_space_before_adjacent_block_comment
                {
                    write!(f, [space()])?;
                }
            } else {
                write!(f, [hard_line_break()])?;
            }
        }
        AnnotationPosition::BlockPostfix => {
            let should_keep_space_before_adjacent_block_comment =
                should_keep_space_before_adjacent_block_comment(flow_facts);
            if flow_facts.is_inline_delimited_block_postfix_star_comment {
                if !flow_facts.render_facts.precedes_separator
                    || !self::inline_block_comment_allows_tight_separator(
                        flow_facts.render_facts.next_character,
                    )
                    || should_keep_space_before_adjacent_block_comment
                {
                    write!(f, [space()])?;
                }
            } else {
                write!(f, [hard_line_break()])?;
            }
        }
        AnnotationPosition::BlockPrefix => {
            let should_keep_space_before_adjacent_block_comment =
                should_keep_space_before_adjacent_block_comment(flow_facts);
            if flow_facts.is_inline_block_star_comment {
                if !flow_facts.render_facts.precedes_separator
                    || !self::inline_block_comment_allows_tight_separator(
                        flow_facts.render_facts.next_character,
                    )
                    || should_keep_space_before_adjacent_block_comment
                {
                    write!(f, [space()])?;
                }
            } else if flow_facts.is_inline_decorator_prefix
                || flow_facts.is_block_prefix_before_type_grouping_operator
            {
                write!(f, [space()])?;
            } else {
                write!(f, [hard_line_break()])?;
            }
        }
        AnnotationPosition::LinePostfixBoundary => {
            let should_keep_inline_slash_separator_comment =
                flow_facts.render_facts.is_slash_comment
                    && flow_facts.render_facts.follows_separator
                    && flow_facts.next_token_is_on_same_line;
            if should_keep_inline_slash_separator_comment {
                write!(f, [space()])?;
            } else if flow_facts.render_facts.is_star_comment && !flow_facts.starts_on_own_line {
                if should_keep_space_before_adjacent_block_comment(flow_facts) {
                    write!(f, [space()])?;
                }
            } else {
                write!(f, [soft_line_break()])?;
            }
        }
    }

    Ok(())
}

/// Return whether spacing should be preserved before a neighboring block comment.
fn should_keep_space_before_adjacent_block_comment(flow_facts: AnnotationFlowFacts) -> bool {
    flow_facts.next_annotation_is_inline_star_comment
        || (flow_facts.render_facts.next_character == Some('/')
            && flow_facts.next_token_is_on_same_line)
}

impl<'ast, T> Format<DestackFormatContext<'ast>> for Annotations<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
{
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let items = collect_annotation_render_items(f.context(), self.position, self.node_id);
        if items.is_empty() {
            return Ok(());
        }

        let mut state = AnnotationRenderState::default();

        for (annotation_index, item) in items.iter().copied().enumerate() {
            // annotation and flow facts
            let annotation = f.context().annotation(item.annotation_id);
            let flow_facts = build_annotation_flow_facts::<T>(
                f.context(),
                self.node_id,
                &items,
                annotation_index,
                item,
                annotation,
            );
            let is_blank_annotation = matches!(annotation, Annotation::Blank { .. });

            // skip repeated blank annotations
            if is_blank_annotation && state.previous_was_blank_annotation {
                continue;
            }

            // keep formatter directives attached to ignored next nodes
            if flow_facts.is_ignore_directive_postfix_comment {
                continue;
            }

            // handle own line postfix boundary slash comments
            if write_boundary_line_postfix_comment(
                f,
                annotation,
                item.position,
                item.annotation_id,
                flow_facts,
            )? {
                continue;
            }

            // handle inline slash line postfix comments via line_postfix
            if write_inline_slash_line_postfix_comment(
                f,
                annotation,
                item.position,
                item.annotation_id,
                flow_facts,
            )? {
                continue;
            }

            // keep formatter directives on own lines for line prefix capture
            if write_line_prefix_ignore_directive_comment(
                f,
                annotation,
                item.position,
                item.annotation_id,
                flow_facts,
            )? {
                continue;
            }

            // spacing before the first rendered annotation in this group
            write_first_annotation_spacing(
                f,
                self.position,
                &mut state,
                item.node_type,
                item.position,
                is_blank_annotation,
                flow_facts,
            )?;

            // suppress separator bound blank markers before own line comments
            if should_skip_blank_annotation_before_own_line_comment(
                item.position,
                is_blank_annotation,
                flow_facts,
            ) {
                state.previous_was_blank_annotation = true;
                continue;
            }

            // emit annotation content
            annotation.format_node(item.annotation_id, f)?;

            if is_blank_annotation {
                state.previous_was_blank_annotation = true;
                continue;
            }

            // spacing after one emitted annotation
            write_annotation_trailing_spacing(f, item.position, flow_facts)?;

            state.previous_was_blank_annotation = false;
        }

        Ok(())
    }
}
