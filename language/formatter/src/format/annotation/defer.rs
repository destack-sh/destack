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
const ANNOTATION_DEFER_RULES: [AnnotationDeferRule; 8] = [
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
pub(super) fn annotation_should_defer<T: Node>(
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
