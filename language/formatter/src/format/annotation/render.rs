use super::facts::token_type_is_comment_trivia;
use super::semicolon::annotation_needs_semicolon_guard_continuation_indent;
use crate::format::directive::{
    comment_node_is_any_ignore_directive, comment_node_is_ignore_directive,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Comment, CommentStyle, Declaration, Doc, DocStyle, Expression, Keyword,
    LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeType, TokenType,
};
use destack_fir::format::{Format, FormatResult, hard_line_break};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotationCapture {
    BlockInfix,
    BlockPrefix,
    BlockPostfix,
    LinePrefix,
    LinePostfix,
    LinePostfixBoundary,
    DelimitedInterior,

    AnyPrefix,
    AnyPostfix,
    AnyPostfixExceptLinePostfixBoundary,
    AnyInfixOrPostfix,
    AnyInfixOrPostfixExceptLinePostfixBoundary,
    DeclarationPrefix,
    DeclarationExportHead,
    DeclarationGenericHead,
    DeclarationBodyHead,
    DeclarationNewHead,
    DeclarationArrowInfix,
    MethodNameInfix,
    MethodParameterHeadInfix,
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
pub(crate) struct AnnotationOutput {
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

impl<'ast> FormatNode<'ast, Doc> for Doc {
    /// Format one documentation comment annotation.
    fn format_node(
        &self,
        node_id: LocalNodeId<Doc>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let raw_comment = f.context().span_str(f.context().span(node_id));
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
        let line = strip_leading_comment_indent(line, common_indent);
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

/// Strip one shared indentation prefix from one multiline comment line.
fn strip_leading_comment_indent(line: &str, indent: usize) -> &str {
    if indent == 0 {
        return line;
    }

    let mut end_index = 0;
    for byte in line.as_bytes().iter().take(indent) {
        if !matches!(*byte, b' ' | b'\t') {
            break;
        }

        end_index += 1;
    }

    &line[end_index..]
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
        let normalized_line = normalize_alignable_block_comment_line(trimmed_line);
        write!(f, [hard_line_break(), space(), text(&normalized_line)])?;
    }

    Ok(())
}

/// Normalize one alignable block-comment line.
fn normalize_alignable_block_comment_line(line: &str) -> String {
    let Some(prefix) = line.strip_suffix("*/") else {
        return line.to_string();
    };

    let prefix = prefix.trim_end();
    if prefix.is_empty() {
        return "*/".to_string();
    }

    format!("{prefix} */")
}

/// Return whether one multiline block comment is alignable on `*` prefixes.
fn block_comment_is_alignable(raw_comment: &str) -> bool {
    raw_comment
        .lines()
        .skip(1)
        .all(|line| line.trim_start_matches('\r').trim_start().starts_with('*'))
}

/// Return whether a separator punctuation immediately follows an annotation.
pub(crate) fn annotation_precedes_separator<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    context
        .annotation_next_non_whitespace_token_type(annotation_id)
        .is_some_and(token_type_is_separator_after_annotation)
}

/// Return whether one token type is a separator for annotation seams.
#[inline]
fn token_type_is_separator_after_annotation(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::Comma
            | TokenType::Semicolon
            | TokenType::LessThan
            | TokenType::OpenParenthesis
            | TokenType::CloseParenthesis
            | TokenType::OpenBracket
            | TokenType::CloseBracket
            | TokenType::CloseBrace
            | TokenType::GreaterThan
            | TokenType::Maybe
            | TokenType::Dot
            | TokenType::Colon
            | TokenType::Assign
            | TokenType::ElementwiseOr
            | TokenType::ElementwiseAnd
    )
}

/// Return whether one token can close a list element that may receive a virtual trailing separator.
#[inline]
fn token_type_is_virtual_trailing_separator_boundary(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::CloseBracket | TokenType::CloseBrace | TokenType::GreaterThan
    )
}

/// Return whether inline block comments can remain tightly bound to one following separator.
#[inline]
fn inline_block_comment_allows_tight_separator(next_token_type: Option<TokenType>) -> bool {
    let Some(next_token_type) = next_token_type else {
        return false;
    };

    matches!(
        next_token_type,
        TokenType::Comma
            | TokenType::Semicolon
            | TokenType::LessThan
            | TokenType::CloseParenthesis
            | TokenType::CloseBracket
            | TokenType::CloseBrace
            | TokenType::GreaterThan
            | TokenType::Maybe
            | TokenType::Dot
            | TokenType::Colon
            | TokenType::Assign
            | TokenType::ElementwiseOr
            | TokenType::ElementwiseAnd
    )
}

/// Return whether the first non-whitespace token after an annotation starts on the same line.
pub(crate) fn annotation_next_token_is_on_same_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation_span = annotation_content_span(context, annotation_id);
    let Some(next_token) = context.annotation_next_non_whitespace_token(annotation_id) else {
        return false;
    };

    let anchor_offset = annotation_span.end.saturating_sub(1);
    context
        .file
        .is_same_line(anchor_offset, next_token.span.start)
}

/// Return whether one annotation is followed by an `else` keyword token.
pub(crate) fn annotation_next_token_is_else_keyword(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    context.annotation_next_token_is_keyword(annotation_id, Keyword::Else)
}

/// Return whether an annotation directly follows a colon in source.
fn annotation_follows_colon<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    context.annotation_previous_non_whitespace_token_type(annotation_id) == Some(TokenType::Colon)
}

/// Return whether one token type is an opening delimiter.
#[inline]
fn token_type_is_opening_delimiter(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::OpenParenthesis
            | TokenType::OpenBracket
            | TokenType::OpenBrace
            | TokenType::LessThan
    )
}

/// Return whether an annotation directly follows an opening delimiter in source.
fn annotation_follows_opening_delimiter<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    context
        .annotation_previous_non_whitespace_token_type(annotation_id)
        .is_some_and(token_type_is_opening_delimiter)
}

/// Return whether one token type is a closing delimiter.
#[inline]
fn token_type_is_closing_delimiter(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::CloseParenthesis
            | TokenType::CloseBracket
            | TokenType::CloseBrace
            | TokenType::GreaterThan
    )
}

/// Return whether one annotation ends right before a closing delimiter in source.
fn annotation_precedes_closing_delimiter<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    context
        .annotation_next_non_whitespace_token_type(annotation_id)
        .is_some_and(token_type_is_closing_delimiter)
}

/// Return whether one annotation is one delimiter-interior infix comment.
fn annotation_is_delimited_interior_comment(
    context: &DestackFormatContext<'_>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if annotation.position() != AnnotationPosition::BlockInfix {
        return false;
    }

    match annotation {
        Annotation::Comment { .. } | Annotation::Doc { .. } | Annotation::Blank { .. } => {}
        _ => return false,
    }

    annotation_follows_opening_delimiter(context, annotation_id)
        && annotation_precedes_closing_delimiter(context, annotation_id)
}

/// Return whether an annotation directly follows a separator in source.
fn annotation_follows_separator<'ast>(
    context: &DestackFormatContext<'ast>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    context.annotation_previous_non_whitespace_token_type(annotation_id) == Some(TokenType::Comma)
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

/// Return whether decorators may stay inline via source-seam signals for this owner node type.
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

/// Return whether one declaration is a `new (...) => ...` function signature.
fn declaration_is_new_signature(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Declaration::Function { signature, .. } = context.tree.get(declaration_id) else {
        return false;
    };

    signature.mode == Some(destack_ast::FunctionMode::New)
}

/// Return whether one declaration uses declaration-generic-head annotation capture.
fn declaration_supports_generic_head_capture(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    matches!(
        context.tree.get(declaration_id),
        Declaration::Type { .. }
            | Declaration::Struct { .. }
            | Declaration::Class { .. }
            | Declaration::Enum { .. }
            | Declaration::Interface { .. }
    )
}

/// Return whether one annotation is a constructor-head seam comment after `new`.
pub(crate) fn annotation_is_declaration_new_head_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(node_id.id);
    if !declaration_is_new_signature(context, declaration_id) {
        return false;
    }

    let Annotation::Comment {
        position: AnnotationPosition::BlockInfix,
        ..
    } = annotation
    else {
        return false;
    };

    context.annotation_next_non_whitespace_token_type(annotation_id)
        == Some(TokenType::OpenParenthesis)
}

/// Return whether one annotation is one lambda-arrow infix seam comment.
pub(crate) fn annotation_is_declaration_arrow_infix_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Declaration {
        return false;
    }

    let Annotation::Comment {
        position: AnnotationPosition::BlockInfix,
        ..
    } = annotation
    else {
        return false;
    };

    !annotation_is_declaration_new_head_comment(context, node_id, annotation, annotation_id)
}

/// Return whether one annotation is one method name seam infix comment.
pub(crate) fn annotation_is_method_name_infix_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Member && T::TYPE != NodeType::Property {
        return false;
    }

    let Annotation::Comment {
        position: AnnotationPosition::BlockInfix,
        ..
    } = annotation
    else {
        return false;
    };

    context.annotation_next_non_whitespace_token_type(annotation_id)
        != Some(TokenType::OpenParenthesis)
}

/// Return whether one annotation is one method parameter-head seam infix comment.
pub(crate) fn annotation_is_method_parameter_head_infix_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Member && T::TYPE != NodeType::Property {
        return false;
    }

    let Annotation::Comment {
        position: AnnotationPosition::BlockInfix,
        ..
    } = annotation
    else {
        return false;
    };

    context.annotation_next_non_whitespace_token_type(annotation_id)
        == Some(TokenType::OpenParenthesis)
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

    context
        .file
        .is_same_line(declaration_span.start, annotation_span.start)
}

/// Return whether one annotation is a generic-head seam comment for a declaration.
pub(crate) fn annotation_is_declaration_generic_head_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(node_id.id);
    if !declaration_supports_generic_head_capture(context, declaration_id) {
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

    let next_token_is_less_than = context.annotation_next_non_whitespace_token_type(annotation_id)
        == Some(TokenType::LessThan);
    if next_token_is_less_than
        && context.annotation_previous_non_whitespace_token_type(annotation_id)
            == Some(TokenType::LessThan)
    {
        return false;
    }

    next_token_is_less_than
        || context.annotation_next_token_is_keyword(annotation_id, Keyword::Extends)
        || context.annotation_next_token_is_keyword(annotation_id, Keyword::Implements)
}

/// Return whether one annotation is a declaration-body seam comment before `{`.
pub(crate) fn annotation_is_declaration_body_head_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
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
    let has_decorator_between_comment_and_body = context
        .annotations(LocalNodeId::<Declaration>::new(node_id.id))
        .is_some_and(|annotation_ids| {
            annotation_ids.into_iter().any(|candidate_id| {
                if candidate_id.id == annotation_id.id {
                    return false;
                }

                let Annotation::Decorator { .. } = context.annotation(candidate_id) else {
                    return false;
                };
                let candidate_span = context.annotation_span(candidate_id);
                candidate_span.start > annotation_span.end
            })
        });
    if has_decorator_between_comment_and_body {
        return false;
    }

    context.annotation_next_non_whitespace_token_type(annotation_id) == Some(TokenType::OpenBrace)
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

    /// Format declaration constructor-head seam comments after `new`.
    #[inline]
    pub fn declaration_new_head_annotations(
        &self,
        node_id: LocalNodeId<Declaration>,
    ) -> Annotations<Declaration> {
        Annotations {
            position: AnnotationCapture::DeclarationNewHead,
            node_id,
        }
    }

    /// Format declaration lambda-arrow infix seam comments.
    #[inline]
    pub fn declaration_arrow_infix_annotations(
        &self,
        node_id: LocalNodeId<Declaration>,
    ) -> Annotations<Declaration> {
        Annotations {
            position: AnnotationCapture::DeclarationArrowInfix,
            node_id,
        }
    }

    /// Format method-name infix seam comments.
    #[inline]
    pub fn method_name_infix_annotations<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::MethodNameInfix,
            node_id,
        }
    }

    /// Format method parameter-head infix seam comments.
    #[inline]
    pub fn method_parameter_head_infix_annotations<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::MethodParameterHeadInfix,
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

    /// Return whether one declaration has any lambda-arrow infix seam comment annotations.
    pub fn has_declaration_arrow_infix_annotation(
        &self,
        node_id: LocalNodeId<Declaration>,
    ) -> bool {
        let Some(annotation_ids) = self.annotations(node_id) else {
            return false;
        };

        annotation_ids.iter().copied().any(|annotation_id| {
            let annotation = self.annotation(annotation_id);
            annotation_is_included_for_capture(
                self,
                AnnotationCapture::DeclarationArrowInfix,
                node_id,
                annotation,
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

    /// Format the line and block postfix annotations for a node, excluding line boundary comments.
    #[inline]
    pub fn any_postfix_except_line_postfix_boundary_annotations<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::AnyPostfixExceptLinePostfixBoundary,
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

    /// Format the line and block infix or postfix annotations for a node, excluding line boundary comments.
    #[inline]
    pub fn any_infix_or_postfix_except_line_postfix_boundary_annotations<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::AnyInfixOrPostfixExceptLinePostfixBoundary,
            node_id,
        }
    }

    /// Format delimiter-interior infix annotations for a node.
    #[inline]
    pub fn delimited_interior_annotations<T: Node>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Annotations<T> {
        Annotations {
            position: AnnotationCapture::DelimitedInterior,
            node_id,
        }
    }

    /// Return whether a node has delimiter-interior infix annotations.
    pub fn has_delimited_interior_annotation<T: Node>(&self, node_id: LocalNodeId<T>) -> bool
    where
        NodeTree: NodeTreeImpl<T>,
    {
        let Some(annotation_ids) = self.annotations(node_id) else {
            return false;
        };

        annotation_ids.iter().copied().any(|annotation_id| {
            annotation_is_delimited_interior_comment(
                self,
                self.annotation(annotation_id),
                annotation_id,
            )
        })
    }
}

/// Return whether capture mode includes this annotation position.
pub(crate) fn annotation_capture_includes_position(
    capture: AnnotationCapture,
    position: AnnotationPosition,
) -> bool {
    match position {
        AnnotationPosition::BlockInfix => matches!(
            capture,
            AnnotationCapture::BlockInfix
                | AnnotationCapture::DelimitedInterior
                | AnnotationCapture::DeclarationNewHead
                | AnnotationCapture::DeclarationArrowInfix
                | AnnotationCapture::MethodNameInfix
                | AnnotationCapture::MethodParameterHeadInfix
                | AnnotationCapture::AnyInfixOrPostfix
                | AnnotationCapture::AnyInfixOrPostfixExceptLinePostfixBoundary
        ),
        AnnotationPosition::BlockPrefix => matches!(
            capture,
            AnnotationCapture::BlockPrefix
                | AnnotationCapture::AnyPrefix
                | AnnotationCapture::DeclarationPrefix
                | AnnotationCapture::DeclarationExportHead
                | AnnotationCapture::DeclarationGenericHead
                | AnnotationCapture::DeclarationBodyHead
        ),
        AnnotationPosition::BlockPostfix => matches!(
            capture,
            AnnotationCapture::BlockPostfix
                | AnnotationCapture::AnyPostfix
                | AnnotationCapture::AnyPostfixExceptLinePostfixBoundary
                | AnnotationCapture::AnyInfixOrPostfix
                | AnnotationCapture::AnyInfixOrPostfixExceptLinePostfixBoundary
        ),
        AnnotationPosition::LinePrefix => matches!(
            capture,
            AnnotationCapture::LinePrefix
                | AnnotationCapture::AnyPrefix
                | AnnotationCapture::DeclarationPrefix
                | AnnotationCapture::DeclarationExportHead
                | AnnotationCapture::DeclarationGenericHead
                | AnnotationCapture::DeclarationBodyHead
        ),
        AnnotationPosition::LinePostfix => matches!(
            capture,
            AnnotationCapture::LinePostfix
                | AnnotationCapture::AnyPostfix
                | AnnotationCapture::AnyPostfixExceptLinePostfixBoundary
                | AnnotationCapture::AnyInfixOrPostfix
                | AnnotationCapture::AnyInfixOrPostfixExceptLinePostfixBoundary
        ),
        AnnotationPosition::LinePostfixBoundary => matches!(
            capture,
            AnnotationCapture::LinePostfixBoundary
                | AnnotationCapture::AnyPostfix
                | AnnotationCapture::AnyInfixOrPostfix
        ),
    }
}

/// Return whether one any-infix-or-postfix capture should drop tagged-template head comments.
fn any_infix_or_postfix_skips_tagged_template_head_comment<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if T::TYPE != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(node_id.id);
    let is_tagged_template_expression = matches!(
        context.tree.get(expression_id),
        Expression::TaggedTemplateExpression { .. }
    );
    if !is_tagged_template_expression {
        return false;
    }

    let is_tagged_template_head_comment = matches!(
        annotation,
        Annotation::Comment {
            position: AnnotationPosition::BlockInfix,
            ..
        }
    ) && context
        .annotation_next_non_whitespace_token_type(annotation_id)
        == Some(TokenType::TemplateString);

    is_tagged_template_head_comment
}

/// Return whether capture mode keeps this declaration annotation in the active stream.
pub(crate) fn annotation_is_included_for_capture<T: Node>(
    context: &DestackFormatContext<'_>,
    capture: AnnotationCapture,
    node_id: LocalNodeId<T>,
    annotation: Annotation,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let position = annotation.position();
    if !annotation_capture_includes_position(capture, position) {
        return false;
    }

    let node_raw_id = node_id.id;
    let is_delimited_interior_comment =
        annotation_is_delimited_interior_comment(context, annotation, annotation_id);

    match capture {
        AnnotationCapture::DeclarationPrefix => {
            !annotation_is_declaration_export_head_comment(
                context,
                LocalNodeId::<T>::new(node_raw_id),
                annotation,
                annotation_id,
            ) && !annotation_is_declaration_generic_head_comment(
                context,
                LocalNodeId::<T>::new(node_raw_id),
                annotation,
                annotation_id,
            ) && !annotation_is_declaration_body_head_comment(
                context,
                LocalNodeId::<T>::new(node_raw_id),
                annotation,
                annotation_id,
            )
        }
        AnnotationCapture::DeclarationExportHead => annotation_is_declaration_export_head_comment(
            context,
            LocalNodeId::<T>::new(node_raw_id),
            annotation,
            annotation_id,
        ),
        AnnotationCapture::DeclarationBodyHead => annotation_is_declaration_body_head_comment(
            context,
            LocalNodeId::<T>::new(node_raw_id),
            annotation,
            annotation_id,
        ),
        AnnotationCapture::DeclarationGenericHead => {
            annotation_is_declaration_generic_head_comment(
                context,
                LocalNodeId::<T>::new(node_raw_id),
                annotation,
                annotation_id,
            )
        }
        AnnotationCapture::DeclarationNewHead => annotation_is_declaration_new_head_comment(
            context,
            LocalNodeId::<T>::new(node_raw_id),
            annotation,
            annotation_id,
        ),
        AnnotationCapture::DeclarationArrowInfix => {
            annotation_is_declaration_arrow_infix_comment(
                context,
                LocalNodeId::<T>::new(node_raw_id),
                annotation,
                annotation_id,
            ) && !is_delimited_interior_comment
        }
        AnnotationCapture::MethodNameInfix => {
            annotation_is_method_name_infix_comment(
                context,
                LocalNodeId::<T>::new(node_raw_id),
                annotation,
                annotation_id,
            ) && !is_delimited_interior_comment
        }
        AnnotationCapture::MethodParameterHeadInfix => {
            annotation_is_method_parameter_head_infix_comment(
                context,
                LocalNodeId::<T>::new(node_raw_id),
                annotation,
                annotation_id,
            )
        }
        AnnotationCapture::DelimitedInterior => is_delimited_interior_comment,
        AnnotationCapture::AnyInfixOrPostfix => {
            !any_infix_or_postfix_skips_tagged_template_head_comment(
                context,
                LocalNodeId::<T>::new(node_raw_id),
                annotation,
                annotation_id,
            ) && !(T::TYPE == NodeType::Pattern && is_delimited_interior_comment)
        }
        AnnotationCapture::AnyInfixOrPostfixExceptLinePostfixBoundary => {
            position != AnnotationPosition::LinePostfixBoundary
                && !any_infix_or_postfix_skips_tagged_template_head_comment(
                    context,
                    LocalNodeId::<T>::new(node_raw_id),
                    annotation,
                    annotation_id,
                )
                && !(T::TYPE == NodeType::Pattern && is_delimited_interior_comment)
        }
        AnnotationCapture::AnyPostfixExceptLinePostfixBoundary => {
            position != AnnotationPosition::LinePostfixBoundary
        }
        _ => true,
    }
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
pub(crate) fn annotation_render_items<'ast, T: Node>(
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
        let (node_type, position) = match annotation {
            Annotation::Blank { position, .. } => (NodeType::Blank, position),
            Annotation::Doc { position, .. } => (NodeType::Doc, position),
            Annotation::Comment { position, .. } => (NodeType::Comment, position),
            Annotation::Decorator { position, .. } => (NodeType::Decorator, position),
        };

        if !annotation_is_included_for_capture(
            context,
            capture,
            LocalNodeId::<T>::new(node_raw_id),
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

/// Reusable flow signals for one annotation item.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AnnotationFlow {
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
    /// Whether this annotation starts on its own line.
    pub(crate) starts_on_own_line: bool,
    /// Whether the next captured annotation is an inline star comment.
    pub(crate) next_annotation_is_inline_star_comment: bool,
    /// Whether the next captured annotation is an inline slash comment.
    pub(crate) next_annotation_is_inline_slash_comment: bool,
    /// Whether the next captured annotation is an own line comment.
    pub(crate) next_annotation_is_own_line_comment: bool,
    /// Whether this and the next annotation should be nestled as adjacent jsdocs.
    pub(crate) next_annotation_should_nestle_jsdoc_comment: bool,
    /// Whether there is an empty line between this and the next annotation in source.
    pub(crate) has_blank_line_before_next_annotation: bool,
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
    /// The next non-whitespace token type after the annotation.
    pub(crate) next_token_type: Option<TokenType>,
    /// Whether the next token after the annotation is `else`.
    pub(crate) next_token_is_else_keyword: bool,
}

/// Return whether there is one empty line between this and the next annotation in source.
fn has_blank_line_before_next_annotation(
    context: &DestackFormatContext<'_>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
) -> bool {
    let Some(current_item) = items.get(annotation_index).copied() else {
        return false;
    };
    let Some(next_item) = items.get(annotation_index + 1).copied() else {
        return false;
    };

    let current_span = context.annotation_span(current_item.annotation_id);
    let next_span = context.annotation_span(next_item.annotation_id);
    let Some(between_span) = current_span.gap_to(next_span) else {
        return false;
    };
    let newline_count = context
        .span_str(between_span)
        .chars()
        .filter(|ch| *ch == '\n')
        .count();
    newline_count >= 2
}

/// Return next-annotation spacing signals in one capture pass.
fn next_annotation_spacing_signals(
    context: &DestackFormatContext<'_>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
) -> (bool, bool, bool, bool) {
    let Some(next_item) = items.get(annotation_index + 1).copied() else {
        return (false, false, false, false);
    };

    let next_starts_on_own_line = annotation_starts_on_own_line(context, next_item.annotation_id);
    let next_annotation_should_nestle_jsdoc_comment =
        should_nestle_adjacent_jsdoc_comments(context, items, annotation_index);

    match context.annotation(next_item.annotation_id) {
        Annotation::Comment { node, .. } => {
            let comment = context.tree.get::<Comment>(node);
            let next_annotation_is_inline_star_comment =
                comment.style == CommentStyle::Star && !next_starts_on_own_line;
            let next_annotation_is_inline_slash_comment =
                comment.style == CommentStyle::Slash && !next_starts_on_own_line;
            let next_annotation_is_own_line_comment = next_starts_on_own_line;
            (
                next_annotation_is_inline_star_comment,
                next_annotation_is_inline_slash_comment,
                next_annotation_is_own_line_comment,
                next_annotation_should_nestle_jsdoc_comment,
            )
        }
        Annotation::Doc { node, .. } => {
            let doc = context.tree.get::<Doc>(node);
            let next_annotation_is_inline_star_comment =
                doc.style == DocStyle::Star && !next_starts_on_own_line;
            let next_annotation_is_inline_slash_comment =
                doc.style == DocStyle::Slash && !next_starts_on_own_line;
            (
                next_annotation_is_inline_star_comment,
                next_annotation_is_inline_slash_comment,
                next_starts_on_own_line,
                next_annotation_should_nestle_jsdoc_comment,
            )
        }
        _ => (
            false,
            false,
            false,
            next_annotation_should_nestle_jsdoc_comment,
        ),
    }
}

/// Return whether one annotation is one multiline jsdoc block comment.
fn annotation_is_multiline_jsdoc_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = context.annotation(annotation_id);
    let annotation_span = context.annotation_span(annotation_id);
    let is_multiline = context.has_newline(annotation_span);
    if !is_multiline {
        return false;
    }

    matches!(
        annotation,
        Annotation::Doc { node, .. } if context.tree.get::<Doc>(node).style == DocStyle::Star
    )
}

/// Return whether two adjacent annotations should be nestled as jsdoc comments.
fn should_nestle_adjacent_jsdoc_comments(
    context: &DestackFormatContext<'_>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
) -> bool {
    let Some(current_item) = items.get(annotation_index).copied() else {
        return false;
    };
    let Some(next_item) = items.get(annotation_index + 1).copied() else {
        return false;
    };

    if !annotation_is_multiline_jsdoc_comment(context, current_item.annotation_id)
        || !annotation_is_multiline_jsdoc_comment(context, next_item.annotation_id)
    {
        return false;
    }

    let current_span = context.annotation_span(current_item.annotation_id);
    let next_span = context.annotation_span(next_item.annotation_id);
    current_span.end == next_span.start
}

/// Return whether the previous rendered annotation is one own-line slash comment.
fn previous_annotation_is_own_line_slash_comment(
    context: &DestackFormatContext<'_>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
) -> bool {
    let Some(previous_item) = annotation_index
        .checked_sub(1)
        .and_then(|previous_index| items.get(previous_index).copied())
    else {
        return false;
    };

    let Annotation::Comment { node, .. } = context.annotation(previous_item.annotation_id) else {
        return false;
    };

    let comment = context.tree.get::<Comment>(node);
    comment.style == CommentStyle::Slash
        && annotation_starts_on_own_line(context, previous_item.annotation_id)
}

/// Build flow signals for one annotation item.
pub(crate) fn annotation_flow<'ast, T: Node>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
    items: &[AnnotationRenderItem],
    annotation_index: usize,
    item: AnnotationRenderItem,
    annotation: Annotation,
) -> AnnotationFlow
where
    NodeTree: NodeTreeImpl<T>,
{
    let is_slash_comment = annotation_is_slash_style(context, &annotation);
    let is_star_comment = annotation_is_star_style(context, &annotation);
    let follows_colon = annotation_follows_colon(context, item.annotation_id);
    let follows_separator = annotation_follows_separator(context, item.annotation_id);
    let follows_opening_delimiter =
        annotation_follows_opening_delimiter(context, item.annotation_id);
    let precedes_separator = annotation_precedes_separator(context, item.annotation_id);
    let next_token_type = context.annotation_next_non_whitespace_token_type(item.annotation_id);
    let starts_on_own_line = annotation_starts_on_own_line(context, item.annotation_id);
    let (
        next_annotation_is_inline_star_comment,
        next_annotation_is_inline_slash_comment,
        next_annotation_is_own_line_comment,
        next_annotation_should_nestle_jsdoc_comment,
    ) = next_annotation_spacing_signals(context, items, annotation_index);
    let has_blank_line_before_next_annotation =
        has_blank_line_before_next_annotation(context, items, annotation_index);
    let previous_annotation_is_own_line_slash_comment =
        previous_annotation_is_own_line_slash_comment(context, items, annotation_index);
    let is_ignore_directive_postfix_comment = annotation_is_ignore_directive_postfix_comment(
        context,
        annotation,
        item.position,
        item.annotation_id,
        is_slash_comment,
        starts_on_own_line,
    );
    let is_ignore_directive_line_prefix_comment =
        annotation_is_ignore_directive_line_prefix_comment(
            context,
            annotation,
            item.position,
            is_slash_comment,
        );

    let next_token_is_on_same_line =
        annotation_next_token_is_on_same_line(context, item.annotation_id);
    let next_token_is_else_keyword =
        annotation_next_token_is_else_keyword(context, item.annotation_id);

    let is_inline_block_star_comment = matches!(
        item.position,
        AnnotationPosition::BlockPrefix | AnnotationPosition::BlockInfix
    ) && is_star_comment
        && next_token_is_on_same_line
        && !follows_colon;
    let is_inline_delimited_block_postfix_star_comment = item.position
        == AnnotationPosition::BlockPostfix
        && is_star_comment
        && next_token_type == Some(TokenType::Comma);
    let inline_block_comment_follows_opening_delimiter =
        is_inline_block_star_comment && follows_opening_delimiter;
    let is_inline_decorator_prefix = annotation_is_inline_decorator_prefix::<T>(
        context,
        node_id,
        annotation,
        item.position,
        item.annotation_id,
        follows_colon,
        follows_opening_delimiter,
        follows_separator,
        previous_annotation_is_own_line_slash_comment,
    );
    AnnotationFlow {
        is_slash_comment,
        is_star_comment,
        follows_colon,
        follows_opening_delimiter,
        follows_separator,
        precedes_separator,
        starts_on_own_line,
        next_annotation_is_inline_star_comment,
        next_annotation_is_inline_slash_comment,
        next_annotation_is_own_line_comment,
        next_annotation_should_nestle_jsdoc_comment,
        has_blank_line_before_next_annotation,
        is_ignore_directive_postfix_comment,
        is_ignore_directive_line_prefix_comment,
        is_inline_block_star_comment,
        is_inline_delimited_block_postfix_star_comment,
        inline_block_comment_follows_opening_delimiter,
        is_inline_decorator_prefix,
        next_token_is_on_same_line,
        next_token_type,
        next_token_is_else_keyword,
    }
}

/// Return whether one annotation is a directive postfix slash comment that should be skipped.
fn annotation_is_ignore_directive_postfix_comment(
    context: &DestackFormatContext<'_>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    is_slash_comment: bool,
    starts_on_own_line: bool,
) -> bool {
    if !is_slash_comment {
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
    is_slash_comment: bool,
) -> bool {
    if !is_slash_comment || position != AnnotationPosition::LinePrefix {
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
    follows_colon: bool,
    follows_opening_delimiter: bool,
    follows_separator: bool,
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

    let decorator_can_stay_inline_for_seam =
        (follows_colon || follows_opening_delimiter || follows_separator)
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

/// Emit slash line-postfix comments through line_postfix to preserve suffix behavior.
pub(crate) fn write_inline_slash_line_postfix_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation: Annotation,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    flow: AnnotationFlow,
) -> FormatResult<bool> {
    if !flow.is_slash_comment {
        return Ok(false);
    }

    // own-line comments should keep normal prefix/postfix spacing semantics
    // line_postfix is only for same-line suffix comments, except decorator seams
    if flow.starts_on_own_line {
        if flow.next_token_type == Some(TokenType::At)
            && matches!(
                position,
                AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
            )
        {
            write!(
                f,
                [
                    hard_line_break(),
                    format_with(|f: &mut DestackFormatter<'ast, '_>| annotation
                        .format_node(annotation_id, f)),
                    hard_line_break()
                ]
            )?;
            return Ok(true);
        }

        return Ok(false);
    }

    let has_virtual_trailing_separator_boundary = flow
        .next_token_type
        .is_some_and(token_type_is_virtual_trailing_separator_boundary);
    let is_line_postfix = position == AnnotationPosition::LinePostfix;
    let is_separator_boundary = position == AnnotationPosition::LinePostfixBoundary
        && (flow.follows_separator
            || flow.next_token_type == Some(TokenType::Comma)
            || has_virtual_trailing_separator_boundary);
    if !is_line_postfix && !is_separator_boundary {
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
    flow: AnnotationFlow,
) -> FormatResult<bool> {
    if position != AnnotationPosition::LinePrefix {
        return Ok(false);
    }

    if !flow.is_ignore_directive_line_prefix_comment {
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

/// One concrete spacing decision for one annotation seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationSpacing {
    None,
    Space,
    HardLine,
    SoftLine,
    EmptyLine,
    ContinuationIndent,
}

/// Emit one concrete annotation spacing decision.
fn write_annotation_spacing<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    spacing: AnnotationSpacing,
) -> FormatResult<()> {
    match spacing {
        AnnotationSpacing::None => {}
        AnnotationSpacing::Space => {
            write!(f, [space()])?;
        }
        AnnotationSpacing::HardLine => {
            write!(f, [hard_line_break()])?;
        }
        AnnotationSpacing::SoftLine => {
            write!(f, [soft_line_break()])?;
        }
        AnnotationSpacing::EmptyLine => {
            write!(f, [empty_line()])?;
        }
        AnnotationSpacing::ContinuationIndent => {
            let indent_width = f.context().options.indent_width as usize;
            let continuation_indent = " ".repeat(indent_width);
            write!(f, [text(&continuation_indent)])?;
        }
    }

    Ok(())
}

/// Return first-spacing decision for one block infix annotation.
fn first_block_infix_spacing(
    capture: AnnotationCapture,
    flow: AnnotationFlow,
) -> AnnotationSpacing {
    if capture == AnnotationCapture::DeclarationArrowInfix {
        if flow.is_inline_block_star_comment && !flow.starts_on_own_line {
            return AnnotationSpacing::Space;
        }

        return AnnotationSpacing::HardLine;
    }

    if flow.is_inline_block_star_comment
        && !flow.inline_block_comment_follows_opening_delimiter
        && !flow.starts_on_own_line
        && capture != AnnotationCapture::DeclarationNewHead
    {
        return AnnotationSpacing::Space;
    }

    if flow.is_inline_block_star_comment
        && !flow.starts_on_own_line
        && capture == AnnotationCapture::AnyInfixOrPostfix
        && flow.inline_block_comment_follows_opening_delimiter
        && flow.next_token_type == Some(TokenType::CloseBrace)
    {
        return AnnotationSpacing::Space;
    }

    if flow.is_inline_block_star_comment {
        return AnnotationSpacing::None;
    }

    AnnotationSpacing::HardLine
}

/// Return first-spacing decision for one block postfix annotation.
fn first_block_postfix_spacing(flow: AnnotationFlow) -> AnnotationSpacing {
    if flow.is_inline_delimited_block_postfix_star_comment && !flow.follows_opening_delimiter {
        return AnnotationSpacing::Space;
    }

    if flow.is_inline_delimited_block_postfix_star_comment {
        return AnnotationSpacing::None;
    }

    AnnotationSpacing::HardLine
}

/// Return first-spacing decision for one block prefix annotation.
fn first_block_prefix_spacing(
    capture: AnnotationCapture,
    flow: AnnotationFlow,
) -> AnnotationSpacing {
    if capture == AnnotationCapture::DeclarationGenericHead
        && flow.is_star_comment
        && !flow.starts_on_own_line
    {
        return AnnotationSpacing::Space;
    }

    let is_block_prefix_after_colon =
        flow.is_star_comment && flow.follows_colon && !flow.starts_on_own_line;

    if flow.is_inline_block_star_comment
        && !flow.inline_block_comment_follows_opening_delimiter
        && !flow.starts_on_own_line
    {
        return AnnotationSpacing::Space;
    }

    if flow.is_inline_block_star_comment {
        return AnnotationSpacing::None;
    }

    if flow.is_inline_decorator_prefix
        && !flow.starts_on_own_line
        && !flow.follows_colon
        && !flow.follows_opening_delimiter
    {
        return AnnotationSpacing::Space;
    }

    if flow.is_inline_decorator_prefix {
        return AnnotationSpacing::None;
    }

    if is_block_prefix_after_colon {
        return AnnotationSpacing::None;
    }

    AnnotationSpacing::HardLine
}

/// Return first-spacing decision for one line postfix annotation.
fn first_line_postfix_spacing(
    position: AnnotationPosition,
    flow: AnnotationFlow,
) -> AnnotationSpacing {
    if position == AnnotationPosition::LinePostfixBoundary && flow.starts_on_own_line {
        return AnnotationSpacing::HardLine;
    }

    AnnotationSpacing::Space
}

/// Return first-spacing decision for one line prefix annotation.
fn first_line_prefix_spacing(
    capture: AnnotationCapture,
    flow: AnnotationFlow,
) -> AnnotationSpacing {
    if flow.is_slash_comment && flow.starts_on_own_line {
        return AnnotationSpacing::HardLine;
    }

    if capture == AnnotationCapture::DeclarationGenericHead {
        return AnnotationSpacing::Space;
    }

    if capture == AnnotationCapture::DeclarationBodyHead && flow.follows_separator {
        return AnnotationSpacing::None;
    }

    if capture == AnnotationCapture::DeclarationBodyHead && flow.starts_on_own_line {
        return AnnotationSpacing::HardLine;
    }

    if capture == AnnotationCapture::DeclarationBodyHead {
        return AnnotationSpacing::Space;
    }

    AnnotationSpacing::None
}

/// Return first-spacing decision for one annotation.
fn first_annotation_spacing(
    capture: AnnotationCapture,
    position: AnnotationPosition,
    flow: AnnotationFlow,
) -> AnnotationSpacing {
    match position {
        AnnotationPosition::BlockInfix => first_block_infix_spacing(capture, flow),
        AnnotationPosition::BlockPostfix => first_block_postfix_spacing(flow),
        AnnotationPosition::BlockPrefix => first_block_prefix_spacing(capture, flow),
        AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary => {
            first_line_postfix_spacing(position, flow)
        }
        AnnotationPosition::LinePrefix => first_line_prefix_spacing(capture, flow),
    }
}

/// Emit spacing before the first annotation in one render group.
pub(crate) fn write_first_annotation_spacing<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    capture: AnnotationCapture,
    state: &mut AnnotationOutput,
    node_type: NodeType,
    position: AnnotationPosition,
    is_blank_annotation: bool,
    flow: AnnotationFlow,
) -> FormatResult<()> {
    if is_blank_annotation {
        return Ok(());
    }

    if state.first_node_type.is_some() {
        return Ok(());
    }

    state.first_node_type = Some(node_type);

    let spacing = first_annotation_spacing(capture, position, flow);
    write_annotation_spacing(f, spacing)
}

/// Return whether one blank annotation should be skipped before an own line comment.
pub(crate) fn should_skip_blank_annotation_before_own_line_comment(
    node_type: NodeType,
    position: AnnotationPosition,
    is_blank_annotation: bool,
    flow: AnnotationFlow,
) -> bool {
    let should_skip_dependency_item_separator_blank = node_type == NodeType::DependencyItem
        && matches!(
            position,
            AnnotationPosition::BlockPrefix
                | AnnotationPosition::BlockPostfix
                | AnnotationPosition::LinePrefix
        );
    let should_skip_block_prefix_blank = position == AnnotationPosition::BlockPrefix;
    let follows_separator_comment = flow.follows_separator
        && (flow.next_annotation_is_own_line_comment
            || flow.next_annotation_is_inline_slash_comment);

    is_blank_annotation
        && (should_skip_block_prefix_blank || should_skip_dependency_item_separator_blank)
        && follows_separator_comment
}

/// Return trailing-spacing decision for one line prefix annotation.
fn trailing_line_prefix_spacing(flow: AnnotationFlow) -> AnnotationSpacing {
    if flow.is_slash_comment && flow.next_token_is_on_same_line {
        return AnnotationSpacing::Space;
    }

    if flow.is_slash_comment {
        return AnnotationSpacing::HardLine;
    }

    if flow.is_star_comment && flow.next_token_is_else_keyword {
        return AnnotationSpacing::Space;
    }

    if flow.next_token_is_on_same_line {
        return AnnotationSpacing::Space;
    }

    AnnotationSpacing::HardLine
}

/// Return trailing-spacing decision for one line postfix annotation.
fn trailing_line_postfix_spacing(flow: AnnotationFlow) -> AnnotationSpacing {
    let keep_space_before_adjacent_block_comment =
        should_keep_space_before_adjacent_block_comment(flow);
    let should_suppress_space = flow.is_star_comment
        && flow.precedes_separator
        && !flow
            .next_token_type
            .is_some_and(token_type_is_slash_like_separator);
    if keep_space_before_adjacent_block_comment || !should_suppress_space {
        return AnnotationSpacing::Space;
    }

    AnnotationSpacing::None
}

/// Return trailing-spacing for one inline star comment seam.
fn trailing_inline_star_comment_spacing(
    flow: AnnotationFlow,
    allows_tight_separator: bool,
) -> AnnotationSpacing {
    let keep_space_before_adjacent_block_comment =
        should_keep_space_before_adjacent_block_comment(flow);
    let should_write_space = should_write_space_after_inline_star_comment(
        flow,
        allows_tight_separator,
        keep_space_before_adjacent_block_comment,
    );
    if should_write_space {
        return AnnotationSpacing::Space;
    }

    AnnotationSpacing::None
}

/// Return trailing-spacing decision for one block infix annotation.
fn trailing_block_infix_spacing(
    capture: AnnotationCapture,
    flow: AnnotationFlow,
) -> AnnotationSpacing {
    if capture == AnnotationCapture::DeclarationArrowInfix {
        if flow.is_inline_block_star_comment {
            return AnnotationSpacing::Space;
        }

        return AnnotationSpacing::HardLine;
    }

    let is_new_head_or_method_parameter_parenthesis_seam = matches!(
        capture,
        AnnotationCapture::DeclarationNewHead | AnnotationCapture::MethodParameterHeadInfix
    ) && flow.next_token_type
        == Some(TokenType::OpenParenthesis);

    if flow.is_inline_block_star_comment {
        let allows_tight_separator =
            inline_block_comment_allows_tight_separator(flow.next_token_type)
                || is_new_head_or_method_parameter_parenthesis_seam;
        return trailing_inline_star_comment_spacing(flow, allows_tight_separator);
    }

    AnnotationSpacing::HardLine
}

/// Return trailing-spacing decision for one block postfix annotation.
fn trailing_block_postfix_spacing(flow: AnnotationFlow) -> AnnotationSpacing {
    if flow.is_inline_delimited_block_postfix_star_comment {
        let allows_tight_separator =
            inline_block_comment_allows_tight_separator(flow.next_token_type);
        return trailing_inline_star_comment_spacing(flow, allows_tight_separator);
    }

    AnnotationSpacing::HardLine
}

/// Return trailing-spacing decision for one block prefix annotation.
fn trailing_block_prefix_spacing(flow: AnnotationFlow) -> AnnotationSpacing {
    if flow.next_annotation_is_own_line_comment && flow.has_blank_line_before_next_annotation {
        return AnnotationSpacing::EmptyLine;
    }

    // decorators stay inline by default, but own-line comments between decorator and owner
    // must preserve their line boundary
    if flow.is_inline_decorator_prefix && flow.next_annotation_is_own_line_comment {
        return AnnotationSpacing::HardLine;
    }

    // keep adjacent inline block comments on one line before the next comment
    if flow.is_star_comment
        && (flow.next_annotation_is_inline_star_comment
            || flow.next_annotation_is_inline_slash_comment)
    {
        return AnnotationSpacing::Space;
    }

    if flow.is_inline_block_star_comment {
        let allows_tight_separator =
            inline_block_comment_allows_tight_separator(flow.next_token_type);
        return trailing_inline_star_comment_spacing(flow, allows_tight_separator);
    }

    if flow.is_inline_decorator_prefix {
        return AnnotationSpacing::Space;
    }

    AnnotationSpacing::HardLine
}

/// Return trailing-spacing decision for one line postfix boundary annotation.
fn trailing_line_postfix_boundary_spacing(flow: AnnotationFlow) -> AnnotationSpacing {
    if flow.is_slash_comment {
        return AnnotationSpacing::HardLine;
    }

    if flow.is_star_comment && !flow.starts_on_own_line {
        if should_keep_space_before_adjacent_block_comment(flow) {
            return AnnotationSpacing::Space;
        }

        return AnnotationSpacing::None;
    }

    AnnotationSpacing::SoftLine
}

/// Return trailing-spacing decision for one annotation.
fn trailing_annotation_spacing(
    capture: AnnotationCapture,
    position: AnnotationPosition,
    flow: AnnotationFlow,
) -> AnnotationSpacing {
    if flow.next_annotation_should_nestle_jsdoc_comment {
        return AnnotationSpacing::None;
    }

    match position {
        AnnotationPosition::LinePrefix => trailing_line_prefix_spacing(flow),
        AnnotationPosition::LinePostfix => trailing_line_postfix_spacing(flow),
        AnnotationPosition::BlockInfix => trailing_block_infix_spacing(capture, flow),
        AnnotationPosition::BlockPostfix => trailing_block_postfix_spacing(flow),
        AnnotationPosition::BlockPrefix => trailing_block_prefix_spacing(flow),
        AnnotationPosition::LinePostfixBoundary => trailing_line_postfix_boundary_spacing(flow),
    }
}

/// Emit spacing after one annotation.
pub(crate) fn write_annotation_trailing_spacing<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    capture: AnnotationCapture,
    position: AnnotationPosition,
    flow: AnnotationFlow,
) -> FormatResult<()> {
    let spacing = trailing_annotation_spacing(capture, position, flow);
    write_annotation_spacing(f, spacing)
}

/// Return spacing emitted immediately before one annotation payload.
fn annotation_content_prefix_spacing(
    context: &DestackFormatContext<'_>,
    position: AnnotationPosition,
    annotation_id: LocalNodeId<Annotation>,
    flow: AnnotationFlow,
) -> AnnotationSpacing {
    if annotation_needs_semicolon_guard_continuation_indent(
        context,
        annotation_id,
        position,
        flow.is_slash_comment,
        flow.starts_on_own_line,
    ) {
        return AnnotationSpacing::ContinuationIndent;
    }

    AnnotationSpacing::None
}

/// Return whether one inline star comment should keep one trailing space.
fn should_write_space_after_inline_star_comment(
    flow: AnnotationFlow,
    allows_tight_separator: bool,
    keep_space_before_adjacent_block_comment: bool,
) -> bool {
    !flow.precedes_separator || !allows_tight_separator || keep_space_before_adjacent_block_comment
}

/// Return whether spacing should be preserved before a neighboring block comment.
#[inline]
fn token_type_is_slash_like_separator(token_type: TokenType) -> bool {
    token_type == TokenType::Divide || token_type_is_comment_trivia(token_type)
}

/// Return whether spacing should be preserved before a neighboring block comment.
fn should_keep_space_before_adjacent_block_comment(flow: AnnotationFlow) -> bool {
    flow.next_annotation_is_inline_star_comment
        || flow.next_annotation_is_inline_slash_comment
        || (flow.next_token_is_on_same_line
            && flow
                .next_token_type
                .is_some_and(token_type_is_comment_trivia))
}

/// Format a prepared annotation render-item list for one capture mode.
fn format_annotation_render_items<'ast, T: Node + Clone>(
    f: &mut DestackFormatter<'ast, '_>,
    capture: AnnotationCapture,
    node_id: LocalNodeId<T>,
    items: &[AnnotationRenderItem],
) -> FormatResult<()>
where
    NodeTree: NodeTreeImpl<T>,
{
    if items.is_empty() {
        return Ok(());
    }

    let mut state = AnnotationOutput::default();

    for (annotation_index, item) in items.iter().copied().enumerate() {
        // annotation and flow signals
        let annotation = f.context().annotation(item.annotation_id);
        let flow = annotation_flow::<T>(
            f.context(),
            node_id.clone(),
            items,
            annotation_index,
            item,
            annotation,
        );
        let is_blank_annotation = matches!(annotation, Annotation::Blank { .. });

        // skip repeated blank annotations
        if is_blank_annotation && state.previous_was_blank_annotation {
            continue;
        }

        // emit ignore directives at postfix seams on their own lines
        if flow.is_ignore_directive_postfix_comment {
            write!(
                f,
                [
                    hard_line_break(),
                    format_with(|f: &mut DestackFormatter<'ast, '_>| annotation
                        .format_node(item.annotation_id, f)),
                    hard_line_break()
                ]
            )?;
            continue;
        }

        // handle inline slash line postfix comments via line_postfix
        if write_inline_slash_line_postfix_comment(
            f,
            annotation,
            item.position,
            item.annotation_id,
            flow,
        )? {
            continue;
        }

        // keep formatter directives on own lines for line prefix capture
        if write_line_prefix_ignore_directive_comment(
            f,
            annotation,
            item.position,
            item.annotation_id,
            flow,
        )? {
            continue;
        }

        // spacing before the first rendered annotation in this group
        write_first_annotation_spacing(
            f,
            capture,
            &mut state,
            item.node_type,
            item.position,
            is_blank_annotation,
            flow,
        )?;

        // suppress separator bound blank markers before own line comments
        if should_skip_blank_annotation_before_own_line_comment(
            item.node_type,
            item.position,
            is_blank_annotation,
            flow,
        ) {
            state.previous_was_blank_annotation = true;
            continue;
        }

        // emit annotation content
        let content_prefix_spacing =
            annotation_content_prefix_spacing(f.context(), item.position, item.annotation_id, flow);
        write_annotation_spacing(f, content_prefix_spacing)?;

        annotation.format_node(item.annotation_id, f)?;

        if is_blank_annotation {
            state.previous_was_blank_annotation = true;
            continue;
        }

        // spacing after one emitted annotation
        write_annotation_trailing_spacing(f, capture, item.position, flow)?;

        state.previous_was_blank_annotation = false;
    }

    Ok(())
}

impl<'ast, T> Format<DestackFormatContext<'ast>> for Annotations<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
{
    /// Format captured annotations at one node and position.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let items = annotation_render_items(f.context(), self.position, self.node_id);
        format_annotation_render_items(f, self.position, self.node_id, &items)
    }
}
