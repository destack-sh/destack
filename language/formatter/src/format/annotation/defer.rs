use crate::DestackFormatContext;
use crate::scan::{
    next_non_whitespace_after_annotation, previous_non_whitespace_before_annotation,
};
use destack_ast::{
    Annotation, AnnotationPosition, Argument, Comment, CommentStyle, Declaration, Expression,
    IfCondition, IfKind, LocalNodeId, Member, Node, NodeTree, NodeTreeImpl, NodeType, Parameter,
    PostfixPosition, Property,
};
use destack_source::Span;

/// Return whether an annotation is followed by an `else` keyword.
pub(super) fn annotation_followed_by_else_keyword(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    annotation_tail_starts_with(context, annotation_id, "else")
}

/// Return whether trailing source after annotation starts with expected text.
fn annotation_tail_starts_with(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    expected: &str,
) -> bool {
    let span = context.get_span::<Annotation>(annotation_id);
    if span.end >= context.file.len {
        return false;
    }

    let tail_span = Span::new(span.file, span.end, context.file.len);
    let Some(tail_source) = context.file.get_span_str(tail_span) else {
        return false;
    };

    tail_source.trim_start().starts_with(expected)
}

/// Return whether the character can terminate a parameter name or pattern.
fn is_parameter_prefix_annotation_left_anchor(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '$' | '?' | ']' | '}' | ')')
}

/// Return whether annotation position is a prefix position.
fn annotation_is_prefix_position(position: AnnotationPosition) -> bool {
    matches!(
        position,
        AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
    )
}

/// Return whether annotation position is declaration-boundary compatible.
fn annotation_is_declaration_boundary_position(position: AnnotationPosition) -> bool {
    matches!(
        position,
        AnnotationPosition::BlockPrefix
            | AnnotationPosition::LinePrefix
            | AnnotationPosition::BlockInfix
    )
}

/// Return whether annotation node is a comment or doc.
fn annotation_is_comment_or_doc(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    matches!(
        context.tree.get::<Annotation>(annotation_id),
        Annotation::Comment { .. } | Annotation::Doc { .. }
    )
}

/// Collect node annotations that match a predicate.
/// Return whether annotation is a slash comment.
fn annotation_is_slash_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };

    let comment = context.tree.get::<Comment>(*node);
    comment.style == CommentStyle::Slash
}

/// Return whether a slash comment starts on its own source line.
fn annotation_is_own_line_slash_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if !annotation_is_slash_comment(context, annotation_id) {
        return false;
    }

    let annotation_span = context.get_span::<Annotation>(annotation_id);
    let head_span = Span::new(annotation_span.file, 0, annotation_span.start);
    let head_source = context.file.get_span_str(head_span).unwrap_or_default();
    let line_start = head_source.rfind('\n').map_or(0, |index| index + 1);
    head_source[line_start..].trim().is_empty()
}

/// Sort annotation ids by source order and deduplicate by annotation spans.
fn deduplicate_annotations_by_span(
    context: &DestackFormatContext<'_>,
    annotation_ids: &mut Vec<LocalNodeId<Annotation>>,
) {
    annotation_ids.sort_by_key(|annotation_id| {
        let span = context.get_span::<Annotation>(*annotation_id);
        (span.file.0, span.start, span.end, annotation_id.id)
    });

    annotation_ids.dedup_by(|left_id, right_id| {
        let left_span = context.get_span::<Annotation>(*left_id);
        let right_span = context.get_span::<Annotation>(*right_id);

        left_span.file == right_span.file
            && left_span.start == right_span.start
            && left_span.end == right_span.end
    });
}

fn collect_matching_annotations<T, P>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    mut predicate: P,
) -> Vec<LocalNodeId<Annotation>>
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
    P: FnMut(
        &DestackFormatContext<'_>,
        LocalNodeId<T>,
        LocalNodeId<Annotation>,
        AnnotationPosition,
    ) -> bool,
{
    let node_index = node_id.id;

    let Some(annotations) = context.get_annotations(node_id) else {
        return Vec::new();
    };

    annotations
        .into_iter()
        .filter(|annotation_id| {
            let annotation = context.tree.get::<Annotation>(*annotation_id);
            let current_node_id = LocalNodeId::<T>::new(node_index);
            predicate(
                context,
                current_node_id,
                *annotation_id,
                annotation.position(),
            )
        })
        .collect()
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

    if !annotation_is_prefix_position(position) {
        return false;
    }

    if !annotation_is_comment_or_doc(context, annotation_id) {
        return false;
    }

    if next_non_whitespace_after_annotation(context, annotation_id) != Some(':') {
        return false;
    }

    previous_non_whitespace_before_annotation(context, annotation_id)
        .is_some_and(is_parameter_prefix_annotation_left_anchor)
}

/// Collect deferred prefix annotations that belong between parameter names and type separators.
pub(crate) fn parameter_type_separator_prefix_annotations(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> Vec<LocalNodeId<Annotation>> {
    collect_matching_annotations(
        context,
        parameter_id,
        is_parameter_type_separator_prefix_annotation,
    )
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
    if !annotation_is_prefix_position(position) {
        return false;
    }

    if !annotation_is_comment_or_doc(context, annotation_id) {
        return false;
    }

    if !annotation_tail_starts_with(context, annotation_id, "=>") {
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
pub(crate) fn is_call_argument_inline_boundary_prefix_annotation<T: Node>(
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

    if !annotation_is_prefix_position(position) {
        return false;
    }

    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };
    let comment = context.tree.get::<Comment>(*node);
    if comment.style != CommentStyle::Slash {
        return false;
    }

    if super::annotation_starts_on_own_line(context, annotation_id) {
        return false;
    }

    if previous_non_whitespace_before_annotation(context, annotation_id) != Some(',') {
        return false;
    }

    let argument_id = LocalNodeId::<Argument>::new(node_id.id);
    argument_is_call_like_dynamic_argument(context, argument_id)
}

/// Collect deferred inline boundary prefix comments for a call-like argument.
pub(crate) fn call_argument_inline_boundary_prefix_annotations(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Vec<LocalNodeId<Annotation>> {
    collect_matching_annotations(
        context,
        argument_id,
        is_call_argument_inline_boundary_prefix_annotation,
    )
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
    if !annotation_is_declaration_boundary_position(position) {
        return false;
    }

    if !annotation_is_comment_or_doc(context, annotation_id) {
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
pub(crate) fn declaration_body_boundary_prefix_annotations(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> Vec<LocalNodeId<Annotation>> {
    collect_matching_annotations(
        context,
        declaration_id,
        is_declaration_body_boundary_prefix_annotation,
    )
}

/// Collect deferred annotations between declaration expression headers and `{`.
pub(crate) fn declaration_expression_body_boundary_prefix_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Annotation>> {
    collect_matching_annotations(
        context,
        expression_id,
        is_declaration_body_boundary_prefix_annotation,
    )
}

/// Return whether annotation position can sit on expression trailing boundaries.
fn annotation_is_expression_trailing_boundary_position(position: AnnotationPosition) -> bool {
    matches!(
        position,
        AnnotationPosition::BlockPostfix
            | AnnotationPosition::LinePostfix
            | AnnotationPosition::LinePostfixBoundary
    )
}

/// Return whether annotation position can sit on if boundary seams.
fn annotation_is_if_boundary_position(position: AnnotationPosition) -> bool {
    annotation_is_expression_trailing_boundary_position(position)
        || matches!(
            position,
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        )
}

/// Return the end of an if condition head.
fn if_condition_head_end(
    context: &DestackFormatContext<'_>,
    condition: &IfCondition,
) -> Option<u32> {
    match condition {
        IfCondition::Expression { condition } => Some(context.get_span(*condition).end),
        IfCondition::Let { declarator, .. } => Some(context.get_span(*declarator).end),
    }
}

/// Return the if condition expression when condition is expression-based.
fn if_condition_expression_id(
    context: &DestackFormatContext<'_>,
    if_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let Expression::If {
        kind, condition, ..
    } = context.tree.get::<Expression>(if_id)
    else {
        return None;
    };
    if *kind == IfKind::Ternary {
        return None;
    }

    let IfCondition::Expression { condition } = condition else {
        return None;
    };

    Some(*condition)
}

/// Return whether annotation span falls between two expression spans.
fn annotation_span_is_between_expressions(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    left_end: u32,
    right_start: u32,
) -> bool {
    let annotation_span = context.get_span::<Annotation>(annotation_id);
    annotation_span.start >= left_end && annotation_span.end <= right_start
}

/// Return whether annotation is between if condition head and then expression body.
fn is_if_head_to_then_boundary_annotation(
    context: &DestackFormatContext<'_>,
    if_id: LocalNodeId<Expression>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool {
    if !annotation_is_if_boundary_position(position) {
        return false;
    }
    if !annotation_is_comment_or_doc(context, annotation_id) {
        return false;
    }
    if annotation_is_own_line_slash_comment(context, annotation_id) {
        return false;
    }

    let Expression::If {
        kind,
        condition,
        then_expression,
        ..
    } = context.tree.get::<Expression>(if_id)
    else {
        return false;
    };
    if *kind == IfKind::Ternary {
        return false;
    }

    let Some(head_end) = if_condition_head_end(context, condition) else {
        return false;
    };
    let then_span = context.get_span(*then_expression);
    annotation_span_is_between_expressions(context, annotation_id, head_end, then_span.start)
}

/// Return whether annotation sits between a condition expression and then body.
fn is_if_condition_expression_to_then_boundary_annotation(
    context: &DestackFormatContext<'_>,
    if_id: LocalNodeId<Expression>,
    condition_id: LocalNodeId<Expression>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool {
    if !annotation_is_if_boundary_position(position) {
        return false;
    }
    if !annotation_is_comment_or_doc(context, annotation_id) {
        return false;
    }
    if annotation_is_own_line_slash_comment(context, annotation_id) {
        return false;
    }
    if previous_non_whitespace_before_annotation(context, annotation_id) != Some(')') {
        return false;
    }

    let Expression::If {
        then_expression, ..
    } = context.tree.get::<Expression>(if_id)
    else {
        return false;
    };

    let condition_span = context.get_span(condition_id);
    let then_span = context.get_span(*then_expression);
    annotation_span_is_between_expressions(
        context,
        annotation_id,
        condition_span.end,
        then_span.start,
    )
}

/// Return whether annotation is between a then expression and its else expression.
fn is_if_then_to_else_boundary_annotation(
    context: &DestackFormatContext<'_>,
    then_id: LocalNodeId<Expression>,
    else_id: LocalNodeId<Expression>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool {
    if !annotation_is_if_boundary_position(position) {
        return false;
    }
    if !annotation_is_comment_or_doc(context, annotation_id) {
        return false;
    }
    if annotation_is_own_line_slash_comment(context, annotation_id) {
        return false;
    }

    let then_span = context.get_span(then_id);
    let else_span = context.get_span(else_id);
    annotation_span_is_between_expressions(context, annotation_id, then_span.end, else_span.start)
}

/// Return whether an if-expression annotation should be deferred to control-flow formatting.
fn should_defer_if_expression_boundary_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression {
        return false;
    }

    let if_id = LocalNodeId::<Expression>::new(node_id.id);
    let Expression::If {
        kind,
        then_expression,
        else_expression,
        ..
    } = context.tree.get::<Expression>(if_id)
    else {
        return false;
    };
    if *kind == IfKind::Ternary {
        return false;
    }

    if is_if_head_to_then_boundary_annotation(context, if_id, annotation_id, position) {
        return true;
    }

    let Some(else_id) = else_expression else {
        return false;
    };
    is_if_then_to_else_boundary_annotation(
        context,
        *then_expression,
        *else_id,
        annotation_id,
        position,
    )
}

/// Return whether a condition-expression annotation should defer to if control rendering.
fn should_defer_if_condition_expression_boundary_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression {
        return false;
    }

    let condition_id = LocalNodeId::<Expression>::new(node_id.id);
    let Some((parent_id, parent_type)) = context.get_parent_by_id(condition_id.id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let if_id = LocalNodeId::<Expression>::new(parent_id);
    let Some(if_condition_id) = if_condition_expression_id(context, if_id) else {
        return false;
    };
    if if_condition_id != condition_id {
        return false;
    }

    is_if_condition_expression_to_then_boundary_annotation(
        context,
        if_id,
        condition_id,
        annotation_id,
        position,
    )
}

/// Return whether a then-expression annotation should be deferred to control-flow formatting.
fn should_defer_if_then_expression_boundary_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression {
        return false;
    }
    if !annotation_is_expression_trailing_boundary_position(position) {
        return false;
    }
    if !annotation_is_comment_or_doc(context, annotation_id) {
        return false;
    }

    let then_id = LocalNodeId::<Expression>::new(node_id.id);
    let Some((parent_id, parent_type)) = context.get_parent_by_id(then_id.id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_if_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::If {
        kind,
        then_expression,
        else_expression,
        ..
    } = context.tree.get::<Expression>(parent_if_id)
    else {
        return false;
    };
    if *kind == IfKind::Ternary {
        return false;
    }
    if *then_expression != then_id {
        return false;
    }

    if is_if_head_to_then_boundary_annotation(context, parent_if_id, annotation_id, position) {
        return true;
    }

    let Some(else_id) = else_expression else {
        return false;
    };
    is_if_then_to_else_boundary_annotation(context, then_id, *else_id, annotation_id, position)
}

/// Return whether an else-expression annotation should be deferred to control-flow formatting.
fn should_defer_if_else_expression_boundary_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if T::TYPE != NodeType::Expression {
        return false;
    }
    if !annotation_is_if_boundary_position(position) {
        return false;
    }
    if !annotation_is_comment_or_doc(context, annotation_id) {
        return false;
    }
    if annotation_is_own_line_slash_comment(context, annotation_id) {
        return false;
    }

    let else_id = LocalNodeId::<Expression>::new(node_id.id);
    let Some((parent_id, parent_type)) = context.get_parent_by_id(else_id.id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_if_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::If {
        kind,
        then_expression,
        else_expression,
        ..
    } = context.tree.get::<Expression>(parent_if_id)
    else {
        return false;
    };
    if *kind == IfKind::Ternary {
        return false;
    }

    let Some(expected_else_id) = else_expression else {
        return false;
    };
    if *expected_else_id != else_id {
        return false;
    }

    is_if_then_to_else_boundary_annotation(
        context,
        *then_expression,
        else_id,
        annotation_id,
        position,
    )
}

/// Return whether annotation should be deferred to if control-flow rendering.
fn should_defer_if_control_flow_boundary_annotation<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    let node_index = node_id.id;

    should_defer_if_expression_boundary_annotation(
        context,
        LocalNodeId::<T>::new(node_index),
        annotation_id,
        position,
    ) || should_defer_if_condition_expression_boundary_annotation(
        context,
        LocalNodeId::<T>::new(node_index),
        annotation_id,
        position,
    ) || should_defer_if_then_expression_boundary_annotation(
        context,
        LocalNodeId::<T>::new(node_index),
        annotation_id,
        position,
    ) || should_defer_if_else_expression_boundary_annotation(
        context,
        LocalNodeId::<T>::new(node_index),
        annotation_id,
        position,
    )
}

/// Collect deferred annotations between if condition head and then expression body.
pub(crate) fn if_head_boundary_annotations(
    context: &DestackFormatContext<'_>,
    if_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Annotation>> {
    let mut annotation_ids = Vec::new();

    if let Some(annotations) = context.get_annotations(if_id) {
        annotation_ids.extend(annotations.into_iter().filter(|annotation_id| {
            let position = context.tree.get::<Annotation>(*annotation_id).position();
            is_if_head_to_then_boundary_annotation(context, if_id, *annotation_id, position)
        }));
    }

    if let Some(condition_id) = if_condition_expression_id(context, if_id)
        && let Some(condition_annotations) = context.get_annotations(condition_id)
    {
        annotation_ids.extend(condition_annotations.into_iter().filter(|annotation_id| {
            let position = context.tree.get::<Annotation>(*annotation_id).position();
            is_if_condition_expression_to_then_boundary_annotation(
                context,
                if_id,
                condition_id,
                *annotation_id,
                position,
            )
        }));
    }

    if let Expression::If {
        then_expression, ..
    } = context.tree.get::<Expression>(if_id)
        && let Some(then_annotations) = context.get_annotations(*then_expression)
    {
        annotation_ids.extend(then_annotations.into_iter().filter(|annotation_id| {
            let position = context.tree.get::<Annotation>(*annotation_id).position();
            is_if_head_to_then_boundary_annotation(context, if_id, *annotation_id, position)
        }));
    }

    deduplicate_annotations_by_span(context, &mut annotation_ids);
    annotation_ids
}

/// Collect deferred annotations between then expression and else expression.
pub(crate) fn if_then_else_boundary_annotations(
    context: &DestackFormatContext<'_>,
    if_id: LocalNodeId<Expression>,
    then_id: LocalNodeId<Expression>,
    else_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Annotation>> {
    let mut annotation_ids = Vec::new();

    if let Some(if_annotations) = context.get_annotations(if_id) {
        for annotation_id in if_annotations {
            let position = context.tree.get::<Annotation>(annotation_id).position();
            if is_if_then_to_else_boundary_annotation(
                context,
                then_id,
                else_id,
                annotation_id,
                position,
            ) {
                annotation_ids.push(annotation_id);
            }
        }
    }

    if let Some(then_annotations) = context.get_annotations(then_id) {
        for annotation_id in then_annotations {
            let position = context.tree.get::<Annotation>(annotation_id).position();
            if is_if_then_to_else_boundary_annotation(
                context,
                then_id,
                else_id,
                annotation_id,
                position,
            ) {
                annotation_ids.push(annotation_id);
            }
        }
    }

    if let Some(else_annotations) = context.get_annotations(else_id) {
        for annotation_id in else_annotations {
            let position = context.tree.get::<Annotation>(annotation_id).position();
            if is_if_then_to_else_boundary_annotation(
                context,
                then_id,
                else_id,
                annotation_id,
                position,
            ) {
                annotation_ids.push(annotation_id);
            }
        }
    }

    deduplicate_annotations_by_span(context, &mut annotation_ids);
    annotation_ids
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
pub(crate) fn should_defer_statement_ternary_boundary_prefix_annotation<T>(
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

/// Ordered rule set for annotation deferral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationDeferRule {
    IfControlFlowBoundary,
    StatementTernaryBoundaryPrefix,
    ParenthesizedBoundary,
    CallBoundary,
    ParameterTypeSeparatorPrefix,
    LambdaArrowPrefix,
    CallArgumentInlineBoundaryPrefix,
    DeclarationBodyBoundaryPrefix,
    MethodBodyBoundary,
}

/// Priority-ordered annotation deferral rules.
const ANNOTATION_DEFER_RULES: [AnnotationDeferRule; 9] = [
    AnnotationDeferRule::IfControlFlowBoundary,
    AnnotationDeferRule::StatementTernaryBoundaryPrefix,
    AnnotationDeferRule::ParenthesizedBoundary,
    AnnotationDeferRule::CallBoundary,
    AnnotationDeferRule::ParameterTypeSeparatorPrefix,
    AnnotationDeferRule::LambdaArrowPrefix,
    AnnotationDeferRule::CallArgumentInlineBoundaryPrefix,
    AnnotationDeferRule::DeclarationBodyBoundaryPrefix,
    AnnotationDeferRule::MethodBodyBoundary,
];

impl AnnotationDeferRule {
    /// Return whether this deferral rule matches the annotation.
    fn matches<T: Node>(
        self,
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<T>,
        annotation_id: LocalNodeId<Annotation>,
        position: AnnotationPosition,
    ) -> bool
    where
        NodeTree: NodeTreeImpl<T>,
    {
        match self {
            AnnotationDeferRule::IfControlFlowBoundary => {
                should_defer_if_control_flow_boundary_annotation(
                    context,
                    node_id,
                    annotation_id,
                    position,
                )
            }
            AnnotationDeferRule::StatementTernaryBoundaryPrefix => {
                should_defer_statement_ternary_boundary_prefix_annotation(
                    context,
                    node_id,
                    annotation_id,
                    position,
                )
            }
            AnnotationDeferRule::ParenthesizedBoundary => {
                should_defer_parenthesized_boundary_annotation(
                    context,
                    node_id,
                    annotation_id,
                    position,
                )
            }
            AnnotationDeferRule::CallBoundary => {
                should_defer_call_boundary_annotation(context, node_id, annotation_id, position)
            }
            AnnotationDeferRule::ParameterTypeSeparatorPrefix => {
                is_parameter_type_separator_prefix_annotation(
                    context,
                    node_id,
                    annotation_id,
                    position,
                )
            }
            AnnotationDeferRule::LambdaArrowPrefix => {
                is_lambda_arrow_prefix_annotation(context, node_id, annotation_id, position)
            }
            AnnotationDeferRule::CallArgumentInlineBoundaryPrefix => {
                is_call_argument_inline_boundary_prefix_annotation(
                    context,
                    node_id,
                    annotation_id,
                    position,
                )
            }
            AnnotationDeferRule::DeclarationBodyBoundaryPrefix => {
                is_declaration_body_boundary_prefix_annotation(
                    context,
                    node_id,
                    annotation_id,
                    position,
                )
            }
            AnnotationDeferRule::MethodBodyBoundary => {
                should_defer_method_body_boundary_annotation(
                    context,
                    node_id,
                    annotation_id,
                    position,
                )
            }
        }
    }
}

/// Return whether annotation emission should be deferred to a specialized formatter.
pub(crate) fn annotation_should_defer<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    annotation_id: LocalNodeId<Annotation>,
    position: AnnotationPosition,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    let node_index = node_id.id;
    let annotation_index = annotation_id.id;

    for rule in ANNOTATION_DEFER_RULES {
        let current_node_id = LocalNodeId::<T>::new(node_index);
        let current_annotation_id = LocalNodeId::<Annotation>::new(annotation_index);
        if rule.matches(context, current_node_id, current_annotation_id, position) {
            return true;
        }
    }

    false
}
