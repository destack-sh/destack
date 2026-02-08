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
    Annotation, AnnotationPosition, Argument, Blank, Comment, CommentStyle, Declaration, Decorator,
    Doc, DocStyle, Expression, IfCondition, IfKind, LocalNodeId, Member, Node, NodeTree,
    NodeTreeImpl, NodeType, Parameter, PostfixPosition, Property,
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

/// Return whether an annotation is followed by an `else` keyword.
fn annotation_followed_by_else_keyword<'ast>(
    context: &DestackFormatContext<'ast>,
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
    tail_source.trim_start().starts_with("else")
}

/// Return whether the character can terminate a parameter name or pattern.
fn is_parameter_prefix_annotation_left_anchor(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '$' | '?' | ']' | '}' | ')')
}

/// Return whether this prefix annotation belongs between parameter name and type separator.
fn is_parameter_type_separator_prefix_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Parameter {
        return false;
    }

    if !matches!(
        position,
        AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
    ) {
        return false;
    }

    if !matches!(
        context.tree.get::<Annotation>(annotation_id),
        Annotation::Comment { .. } | Annotation::Doc { .. }
    ) {
        return false;
    }

    if next_non_whitespace_after_annotation(context, annotation_id) != Some(':') {
        return false;
    }

    previous_non_whitespace_before_annotation(context, annotation_id)
        .is_some_and(is_parameter_prefix_annotation_left_anchor)
}

/// Collect deferred prefix annotations that belong between parameter names and type separators.
pub(crate) fn parameter_type_separator_prefix_annotations<'ast>(
    context: &DestackFormatContext<'ast>,
    parameter_id: LocalNodeId<Parameter>,
) -> Vec<LocalNodeId<Annotation>> {
    let Some(annotations) = context.get_annotations(parameter_id) else {
        return Vec::new();
    };

    annotations
        .into_iter()
        .filter(|annotation_id| {
            let annotation = context.tree.get::<Annotation>(*annotation_id);
            is_parameter_type_separator_prefix_annotation(
                context,
                parameter_id,
                *annotation_id,
                annotation.position(),
            )
        })
        .collect()
}

/// Return whether this prefix annotation belongs between lambda parameters and arrow token.
pub(crate) fn is_lambda_arrow_prefix_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if !matches!(
        position,
        AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
    ) {
        return false;
    }

    if !matches!(
        context.tree.get::<Annotation>(annotation_id),
        Annotation::Comment { .. } | Annotation::Doc { .. }
    ) {
        return false;
    }

    let span = context.get_span::<Annotation>(annotation_id);
    if span.end >= context.file.len {
        return false;
    }

    let tail_span = Span::new(span.file, span.end, context.file.len);
    let Some(tail_source) = context.file.get_span_str(tail_span) else {
        return false;
    };
    if !tail_source.trim_start().starts_with("=>") {
        return false;
    }

    previous_non_whitespace_before_annotation(context, annotation_id)
        .is_some_and(is_parameter_prefix_annotation_left_anchor)
}

/// Return whether this argument belongs to call-like dynamic argument lists.
fn argument_is_call_like_dynamic_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_expression_id) {
        Expression::Call {
            dynamic_arguments, ..
        } => dynamic_arguments.contains(&argument_id),
        Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments.contains(&argument_id),
        _ => false,
    }
}

/// Return whether this prefix slash comment should be deferred to call argument separators.
fn is_call_argument_inline_boundary_prefix_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Argument {
        return false;
    }

    if !matches!(
        position,
        AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
    ) {
        return false;
    }

    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };
    let comment = context.tree.get::<Comment>(*node);
    if comment.style != CommentStyle::Slash {
        return false;
    }

    if annotation_starts_on_own_line(context, annotation_id) {
        return false;
    }

    if previous_non_whitespace_before_annotation(context, annotation_id) != Some(',') {
        return false;
    }

    let argument_id = LocalNodeId::<Argument>::new(node_id.id);
    argument_is_call_like_dynamic_argument(context, argument_id)
}

/// Collect deferred inline boundary prefix comments for a call-like argument.
pub(crate) fn call_argument_inline_boundary_prefix_annotations<'ast>(
    context: &DestackFormatContext<'ast>,
    argument_id: LocalNodeId<Argument>,
) -> Vec<LocalNodeId<Annotation>> {
    let Some(annotations) = context.get_annotations(argument_id) else {
        return Vec::new();
    };

    annotations
        .into_iter()
        .filter(|annotation_id| {
            let annotation = context.tree.get::<Annotation>(*annotation_id);
            is_call_argument_inline_boundary_prefix_annotation(
                context,
                argument_id,
                *annotation_id,
                annotation.position(),
            )
        })
        .collect()
}

/// Return whether this declaration has a braced body after its header.
fn declaration_has_braced_body(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    matches!(
        context.tree.get(declaration_id),
        Declaration::Global { .. }
            | Declaration::Struct { .. }
            | Declaration::Class { .. }
            | Declaration::Enum { .. }
            | Declaration::Interface { .. }
            | Declaration::Extension { .. }
    )
}

/// Return whether this annotation belongs between declaration headers and `{`.
fn is_declaration_body_boundary_prefix_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if !matches!(
        position,
        AnnotationPosition::BlockPrefix
            | AnnotationPosition::LinePrefix
            | AnnotationPosition::BlockInfix
    ) {
        return false;
    }

    if !matches!(
        context.tree.get::<Annotation>(annotation_id),
        Annotation::Comment { .. } | Annotation::Doc { .. }
    ) {
        return false;
    }

    let declaration_id = if T::TYPE == NodeType::Declaration {
        LocalNodeId::<Declaration>::new(node_id.id)
    } else if T::TYPE == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(node_id.id);
        let Expression::Declaration(declaration_id) = context.tree.get::<Expression>(expression_id)
        else {
            return false;
        };
        *declaration_id
    } else {
        return false;
    };

    declaration_has_braced_body(context, declaration_id)
        && next_non_whitespace_after_annotation(context, annotation_id) == Some('{')
}

/// Collect deferred prefix annotations between declaration headers and `{`.
pub(crate) fn declaration_body_boundary_prefix_annotations<'ast>(
    context: &DestackFormatContext<'ast>,
    declaration_id: LocalNodeId<Declaration>,
) -> Vec<LocalNodeId<Annotation>> {
    let Some(annotations) = context.get_annotations(declaration_id) else {
        return Vec::new();
    };

    annotations
        .into_iter()
        .filter(|annotation_id| {
            let annotation = context.tree.get::<Annotation>(*annotation_id);
            is_declaration_body_boundary_prefix_annotation(
                context,
                declaration_id,
                *annotation_id,
                annotation.position(),
            )
        })
        .collect()
}

/// Collect deferred annotations between declaration expression headers and `{`.
pub(crate) fn declaration_expression_body_boundary_prefix_annotations<'ast>(
    context: &DestackFormatContext<'ast>,
    expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Annotation>> {
    let Some(annotations) = context.get_annotations(expression_id) else {
        return Vec::new();
    };

    annotations
        .into_iter()
        .filter(|annotation_id| {
            let annotation = context.tree.get::<Annotation>(*annotation_id);
            is_declaration_body_boundary_prefix_annotation(
                context,
                expression_id,
                *annotation_id,
                annotation.position(),
            )
        })
        .collect()
}

/// Return whether parameter type separator prefix annotations should be deferred to parameter output.
fn should_defer_parameter_type_separator_prefix_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    is_parameter_type_separator_prefix_annotation(context, node_id, annotation_id, position)
}

/// Return whether lambda arrow boundary prefix annotations should be deferred to lambda output.
fn should_defer_lambda_arrow_prefix_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    is_lambda_arrow_prefix_annotation(context, node_id, annotation_id, position)
}

/// Return whether call argument separator comments should be deferred to call rendering.
fn should_defer_call_argument_inline_boundary_prefix_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    is_call_argument_inline_boundary_prefix_annotation(context, node_id, annotation_id, position)
}

/// Return whether declaration body boundary prefix annotations should be deferred.
fn should_defer_declaration_body_boundary_prefix_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    is_declaration_body_boundary_prefix_annotation(context, node_id, annotation_id, position)
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

/// Return whether an annotation starts on a line with only leading whitespace.
fn annotation_starts_on_own_line<'ast>(
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

/// Return whether a span falls on a ternary separator boundary.
fn ternary_contains_boundary_annotation_span(
    context: &DestackFormatContext<'_>,
    ternary_id: LocalNodeId<Expression>,
    annotation_span: Span,
) -> bool {
    let Expression::If {
        kind: IfKind::Ternary,
        condition,
        then_expression,
        else_expression,
        ..
    } = context.tree.get(ternary_id)
    else {
        return false;
    };

    let IfCondition::Expression { condition } = condition else {
        return false;
    };

    let condition_span = context.get_span(*condition);
    let then_span = context.get_span(*then_expression);

    if annotation_span.start >= condition_span.end && annotation_span.end <= then_span.start {
        return true;
    }

    let Some(else_id) = else_expression else {
        return false;
    };
    let else_span = context.get_span(*else_id);
    if annotation_span.start >= then_span.end && annotation_span.end <= else_span.start {
        return true;
    }

    matches!(
        context.tree.get(*else_id),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    ) && ternary_contains_boundary_annotation_span(context, *else_id, annotation_span)
}

/// Return whether a statement-level line-prefix star comment should be deferred to ternary output.
fn should_defer_statement_ternary_boundary_prefix_annotation<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression || position != AnnotationPosition::LinePrefix {
        return false;
    }

    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };
    let comment = context.tree.get::<Comment>(*node);
    if comment.style != CommentStyle::Star {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(node_id.id);
    let Expression::Statement(inner_expression_id) = context.tree.get::<Expression>(expression_id)
    else {
        return false;
    };
    if !matches!(
        context.tree.get::<Expression>(*inner_expression_id),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    ) {
        return false;
    }

    let annotation_span = context.get_span::<Annotation>(annotation_id);
    let inner_span = context.get_span::<Expression>(*inner_expression_id);
    if annotation_span.start <= inner_span.start || annotation_span.end >= inner_span.end {
        return false;
    }

    ternary_contains_boundary_annotation_span(context, *inner_expression_id, annotation_span)
}

/// Return whether a postfix star comment should be deferred after a parenthesized close.
fn should_defer_parenthesized_boundary_annotation<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression {
        return false;
    }
    if !matches!(
        position,
        AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
    ) {
        return false;
    }

    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };
    let comment = context.tree.get::<Comment>(*node);
    if comment.style != CommentStyle::Star {
        return false;
    }

    if next_non_whitespace_after_annotation(context, annotation_id) != Some(')') {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(node_id.id);
    let Some((parent_id, parent_type)) = context.get_parent_by_id(node_id.id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    if matches!(
        context.tree.get::<Expression>(parent_id),
        Expression::Parenthesized { expression } if *expression == expression_id
    ) {
        return true;
    }

    let Expression::Binary { right, .. } = context.tree.get::<Expression>(parent_id) else {
        return false;
    };
    if *right != expression_id {
        return false;
    }

    let annotation_span = context.get_span::<Annotation>(annotation_id);
    let binary_span = context.get_span::<Expression>(parent_id);
    if annotation_span.start < binary_span.end {
        return false;
    }

    let Some((grand_parent_id, grand_parent_type)) = context.get_parent_by_id(parent_id.id) else {
        return false;
    };
    if grand_parent_type != NodeType::Expression {
        return false;
    }

    let grand_parent_id = LocalNodeId::<Expression>::new(grand_parent_id);
    matches!(
        context.tree.get::<Expression>(grand_parent_id),
        Expression::Parenthesized { expression } if *expression == parent_id
    )
}

/// Return whether a callee boundary comment should be deferred to call rendering.
fn should_defer_call_boundary_annotation<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(node_id.id);
    let Some(call_position) = enclosing_empty_call_position_for_callee(context, expression_id)
    else {
        return false;
    };

    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };
    let comment = context.tree.get::<Comment>(*node);

    if comment.style == CommentStyle::Star
        && position == AnnotationPosition::BlockPostfix
        && previous_non_whitespace_before_annotation(context, annotation_id) == Some('(')
        && next_non_whitespace_after_annotation(context, annotation_id) == Some(')')
    {
        return true;
    }

    if comment.style != CommentStyle::Slash {
        return false;
    }

    let has_line_postfix_position = matches!(
        position,
        AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
    );
    let has_block_postfix_position = position == AnnotationPosition::BlockPostfix;

    // keep empty call argument line comments inside `()`
    if (has_line_postfix_position || has_block_postfix_position)
        && previous_non_whitespace_before_annotation(context, annotation_id) == Some('(')
        && next_non_whitespace_after_annotation(context, annotation_id) == Some(')')
    {
        return true;
    }

    // keep optional call boundary comments at the call expression level
    has_line_postfix_position
        && (call_position == PostfixPosition::Indirect
            || next_non_whitespace_after_annotation(context, annotation_id) == Some('?'))
        && next_non_whitespace_after_annotation(context, annotation_id) == Some('?')
}

/// Return the method body expression when `expression_id` is a method return type node.
fn method_body_for_signature_return_type_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let (parent_id, parent_type) = context.get_parent_by_id(expression_id.id)?;
    match parent_type {
        NodeType::Member => {
            let member_id = LocalNodeId::<Member>::new(parent_id);
            let Member::Method {
                signature, body, ..
            } = context.tree.get(member_id)
            else {
                return None;
            };
            if signature.return_type == Some(expression_id) {
                *body
            } else {
                None
            }
        }
        NodeType::Property => {
            let property_id = LocalNodeId::<Property>::new(parent_id);
            let Property::Method {
                signature, body, ..
            } = context.tree.get(property_id)
            else {
                return None;
            };
            if signature.return_type == Some(expression_id) {
                *body
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Return whether expression is the body of a method member/property.
fn expression_is_method_body(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent_by_id(expression_id.id) else {
        return false;
    };

    match parent_type {
        NodeType::Member => {
            let member_id = LocalNodeId::<Member>::new(parent_id);
            let Member::Method { body, .. } = context.tree.get(member_id) else {
                return false;
            };
            *body == Some(expression_id)
        }
        NodeType::Property => {
            let property_id = LocalNodeId::<Property>::new(parent_id);
            let Property::Method { body, .. } = context.tree.get(property_id) else {
                return false;
            };
            *body == Some(expression_id)
        }
        _ => false,
    }
}

/// Return whether a method boundary slash comment should be deferred to method body rendering.
fn should_defer_method_body_boundary_annotation<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression {
        return false;
    }

    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };
    let comment = context.tree.get::<Comment>(*node);
    if comment.style != CommentStyle::Slash {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(node_id.id);
    let expression = context.tree.get::<Expression>(expression_id);

    if position == AnnotationPosition::LinePostfixBoundary
        && method_body_for_signature_return_type_expression(context, expression_id).is_some_and(
            |body_id| {
                matches!(
                    context.tree.get::<Expression>(body_id),
                    Expression::Block(_)
                )
            },
        )
        && next_non_whitespace_after_annotation(context, annotation_id) == Some('{')
    {
        return true;
    }

    position == AnnotationPosition::BlockPrefix
        && matches!(expression, Expression::Block(_))
        && expression_is_method_body(context, expression_id)
}

/// Return the enclosing empty call position when expression belongs to call callee wrappers.
fn enclosing_empty_call_position_for_callee(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<PostfixPosition> {
    let mut current_id = expression_id;

    loop {
        let (parent_id, parent_type) = context.get_parent_by_id(current_id.id)?;
        if parent_type != NodeType::Expression {
            return None;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get::<Expression>(parent_id) {
            Expression::Call {
                left,
                position,
                dynamic_arguments,
                ..
            } => {
                if *left != current_id || !dynamic_arguments.is_empty() {
                    return None;
                }

                return Some(*position);
            }
            Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
                if *left != current_id {
                    return None;
                }

                current_id = parent_id;
            }
            Expression::Parenthesized { expression } => {
                if *expression != current_id {
                    return None;
                }

                current_id = parent_id;
            }
            _ => return None,
        }
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
            let annotation = f.context().tree.get::<Annotation>(annotation_id);
            let (node_type, position) = match annotation {
                Annotation::Blank { position, .. } => (NodeType::Blank, *position),
                Annotation::Doc { position, .. } => (NodeType::Doc, *position),
                Annotation::Comment { position, .. } => (NodeType::Comment, *position),
                Annotation::Decorator { position, .. } => (NodeType::Decorator, *position),
            };
            // defer statement level ternary boundary comments to the ternary formatter
            if should_defer_statement_ternary_boundary_prefix_annotation(
                f.context(),
                self.node_id,
                annotation_id,
                position,
            ) {
                continue;
            }
            if should_defer_parenthesized_boundary_annotation(
                f.context(),
                self.node_id,
                annotation_id,
                position,
            ) {
                continue;
            }
            if should_defer_call_boundary_annotation(
                f.context(),
                self.node_id,
                annotation_id,
                position,
            ) {
                continue;
            }
            if should_defer_parameter_type_separator_prefix_annotation(
                f.context(),
                self.node_id,
                annotation_id,
                position,
            ) {
                continue;
            }
            if should_defer_lambda_arrow_prefix_annotation(
                f.context(),
                self.node_id,
                annotation_id,
                position,
            ) {
                continue;
            }
            if should_defer_call_argument_inline_boundary_prefix_annotation(
                f.context(),
                self.node_id,
                annotation_id,
                position,
            ) {
                continue;
            }
            if should_defer_declaration_body_boundary_prefix_annotation(
                f.context(),
                self.node_id,
                annotation_id,
                position,
            ) {
                continue;
            }
            if should_defer_method_body_boundary_annotation(
                f.context(),
                self.node_id,
                annotation_id,
                position,
            ) {
                continue;
            }

            // filter annotation
            let is_included = match position {
                AnnotationPosition::BlockInfix => {
                    self.position == AnnotationCapture::BlockInfix
                        || self.position == AnnotationCapture::AnyInfixOrPostfix
                }
                AnnotationPosition::BlockPrefix => {
                    self.position == AnnotationCapture::BlockPrefix
                        || self.position == AnnotationCapture::AnyPrefix
                }
                AnnotationPosition::BlockPostfix => {
                    self.position == AnnotationCapture::BlockPostfix
                        || self.position == AnnotationCapture::AnyPostfix
                        || self.position == AnnotationCapture::AnyInfixOrPostfix
                }
                AnnotationPosition::LinePrefix => {
                    self.position == AnnotationCapture::LinePrefix
                        || self.position == AnnotationCapture::AnyPrefix
                }
                AnnotationPosition::LinePostfix => {
                    self.position == AnnotationCapture::LinePostfix
                        || self.position == AnnotationCapture::AnyPostfix
                        || self.position == AnnotationCapture::AnyInfixOrPostfix
                }
                AnnotationPosition::LinePostfixBoundary => {
                    self.position == AnnotationCapture::LinePostfixBoundary
                        || self.position == AnnotationCapture::AnyPostfix
                        || self.position == AnnotationCapture::AnyInfixOrPostfix
                }
            };
            if !is_included {
                continue;
            }

            // for line comments (// style), use line_postfix to defer to end of line
            // this keeps `x, // comment` together; block comments stay inline
            let is_slash_comment = if let Annotation::Comment {
                node: comment_id, ..
            } = annotation
            {
                let comment = f.context().tree.get::<Comment>(*comment_id);
                comment.style == CommentStyle::Slash
            } else {
                false
            };
            let is_star_comment = if let Annotation::Comment {
                node: comment_id, ..
            } = annotation
            {
                let comment = f.context().tree.get::<Comment>(*comment_id);
                comment.style == CommentStyle::Star
            } else {
                false
            };
            let comment_starts_on_own_line = if is_slash_comment {
                comment_annotation_starts_on_own_line(f.context(), annotation_id)
            } else {
                false
            };
            let is_ignore_directive_postfix_comment = is_slash_comment
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

                    let is_own_line_postfix =
                        annotation_starts_on_own_line(f.context(), annotation_id)
                            || annotation_has_leading_newline(f.context(), annotation_id)
                            || comment_starts_on_own_line;

                    if is_own_line_postfix {
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

            let has_if_ancestor = f.context().get_ancestors(self.node_id).into_iter().any(
                |(ancestor_id, node_type)| {
                    if node_type != NodeType::Expression {
                        return false;
                    }
                    matches!(
                        f.context()
                            .tree
                            .get::<Expression>(LocalNodeId::<Expression>::new(ancestor_id)),
                        Expression::If { .. }
                    )
                },
            );
            let has_member_ancestor = f
                .context()
                .get_ancestors(self.node_id)
                .into_iter()
                .any(|(_, node_type)| node_type == NodeType::Member);
            let node_has_member_shape = if T::TYPE == NodeType::Expression {
                match f
                    .context()
                    .tree
                    .get::<Expression>(LocalNodeId::<Expression>::new(self.node_id.id))
                {
                    Expression::Member { .. } | Expression::PrivateMember { .. } => true,
                    Expression::Path { path, .. } => path.segments.len() > 1,
                    _ => false,
                }
            } else {
                false
            };
            let has_member_context = has_member_ancestor || node_has_member_shape;
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
                && has_if_ancestor
                && annotation_followed_by_else_keyword(f.context(), annotation_id)
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
            ) && is_star_comment
                && !annotation_starts_on_own_line(f.context(), annotation_id)
                && !annotation_follows_colon(f.context(), annotation_id);
            let is_inline_delimited_block_postfix_star_comment = position
                == AnnotationPosition::BlockPostfix
                && is_star_comment
                && annotation_follows_opening_delimiter(f.context(), annotation_id)
                && annotation_precedes_separator(f.context(), annotation_id);
            let inline_block_comment_follows_opening_delimiter = is_inline_block_star_comment
                && annotation_follows_opening_delimiter(f.context(), annotation_id);

            if is_slash_comment
                && matches!(
                    position,
                    AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
                )
            {
                let starts_on_own_line = annotation_starts_on_own_line(f.context(), annotation_id)
                    || annotation_has_leading_newline(f.context(), annotation_id)
                    || comment_starts_on_own_line;
                let next_character =
                    next_non_whitespace_after_annotation(f.context(), annotation_id);
                let is_member_chain_boundary =
                    has_member_context && matches!(next_character, Some('.' | '?'));
                let should_preserve_own_line_indentation = starts_on_own_line
                    && (is_member_chain_boundary
                        || !annotation_precedes_separator(f.context(), annotation_id)
                        || matches!(next_character, Some(';' | '(')));
                if should_preserve_own_line_indentation {
                    let raw_line = annotation_line_with_indentation(f.context(), annotation_id)
                        .unwrap_or_else(|| {
                            let annotation_span = f.context().get_span::<Annotation>(annotation_id);
                            let annotation_source = f.context().get_span_str(annotation_span);
                            annotation_source.trim_end().to_string()
                        });
                    write_annotation_line_with_indentation(f, raw_line.as_str())?;
                    continue;
                }

                let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(f, [space()])?;
                    if has_if_ancestor {
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

            if is_slash_comment
                && position == AnnotationPosition::BlockPostfix
                && !has_member_context
            {
                let starts_on_own_line = annotation_starts_on_own_line(f.context(), annotation_id)
                    || annotation_has_leading_newline(f.context(), annotation_id)
                    || comment_starts_on_own_line;
                let next_character =
                    next_non_whitespace_after_annotation(f.context(), annotation_id);
                let should_preserve_own_line_indentation = starts_on_own_line
                    && (!annotation_precedes_separator(f.context(), annotation_id)
                        || matches!(next_character, Some(';' | '(')));
                if should_preserve_own_line_indentation {
                    let raw_line = annotation_line_with_indentation(f.context(), annotation_id)
                        .unwrap_or_else(|| {
                            let annotation_span = f.context().get_span::<Annotation>(annotation_id);
                            let annotation_source = f.context().get_span_str(annotation_span);
                            annotation_source.trim_end().to_string()
                        });
                    write_annotation_line_with_indentation(f, raw_line.as_str())?;
                    continue;
                }
            }

            if is_slash_comment && position == AnnotationPosition::BlockPrefix {
                let starts_on_own_line = annotation_starts_on_own_line(f.context(), annotation_id)
                    || annotation_has_leading_newline(f.context(), annotation_id)
                    || comment_starts_on_own_line;
                let should_preserve_member_chain_prefix_line =
                    has_member_context && starts_on_own_line;
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
                    && has_member_context
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
                    && has_if_ancestor
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
                if !annotation_precedes_separator(f.context(), annotation_id) {
                    write!(f, [space()])?;
                }
                continue;
            }

            // insert space/newline for first annotation in group
            if first_node_type.is_none() {
                first_node_type = Some(node_type);
                let is_block_prefix_after_colon = position == AnnotationPosition::BlockPrefix
                    && is_star_comment
                    && annotation_follows_colon(f.context(), annotation_id);

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
                            if !annotation_follows_opening_delimiter(f.context(), annotation_id) {
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
                && matches!(
                    next_non_whitespace_after_annotation(f.context(), annotation_id),
                    Some('|' | '&')
                );

            // format annotation itself
            annotation.format_node(annotation_id, f)?;

            // insert space / newline
            match position {
                AnnotationPosition::LinePrefix => {
                    write!(f, [space()])?;
                }
                AnnotationPosition::LinePostfix => {
                    if !(is_star_comment
                        && annotation_precedes_separator(f.context(), annotation_id))
                    {
                        write!(f, [space()])?;
                    }
                }
                AnnotationPosition::BlockInfix => {
                    if is_inline_block_star_comment {
                        if !annotation_precedes_separator(f.context(), annotation_id) {
                            write!(f, [space()])?;
                        }
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                AnnotationPosition::BlockPostfix => {
                    if is_inline_delimited_block_postfix_star_comment {
                        if !annotation_precedes_separator(f.context(), annotation_id) {
                            write!(f, [space()])?;
                        }
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }
                AnnotationPosition::BlockPrefix => {
                    if is_inline_block_star_comment {
                        if !annotation_precedes_separator(f.context(), annotation_id) {
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
                    if !(is_star_comment
                        && annotation_precedes_separator(f.context(), annotation_id))
                    {
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
                    .get_ancestors(node_id)
                    .into_iter()
                    .find(|(_, node_type)| *node_type == NodeType::Declaration);
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
        let expression_contains_inline_comment =
            expression_source.contains("//") || expression_source.contains("/*");
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
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::DeclarationDescriptor;

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
            |p| p.eat_expression(),
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
            |p| p.eat_expression(),
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
            |p| p.eat_expression(),
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
            |p| p.eat_expression(),
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
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Pure hint comments should keep compact style.
    #[test]
    fn test_format_compact_pure_hint_comment() {
        assert_format!(
            "/*#__PURE__*/factory()",
            "/*#__PURE__*/ factory()",
            |p| p.eat_expression(),
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
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }
}
