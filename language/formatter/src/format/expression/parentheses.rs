use crate::format::analysis::{next_non_whitespace_after_span, timing};
use crate::format::chain::flatten_type_binary_expression;
use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, BinaryOperator, Declaration, DestackFormatContext,
    Expression, FunctionKind, IfKind, LocalNodeId, NodeTree, NodeType, PostfixPosition,
    TypeBinaryOperator, needs_parens_in_postfix_position, tree_literal_should_expand,
};
use crate::format::operator::{is_simple_type_binary_left_expression, is_type_context};
use destack_ast::{Comment, CommentStyle};
use destack_source::Span;

/// Parenthesized unwrap mode for expression contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParenthesizedUnwrapMode {
    /// Unwrap when the value is used as a member object.
    MemberObject,
    /// Unwrap when the value is a `new` callee wrapper.
    NewMemberCallee,
}

/// Parenthesized drop mode for type contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParenthesizedDropMode {
    /// Drop wrappers in generic expression contexts.
    ExpressionWrapper,
    /// Drop wrappers around type-binary left operands.
    TypeBinaryLeft { node_id: LocalNodeId<Expression> },
}

/// Decide whether a parenthesized expression should unwrap for a mode.
pub(crate) fn should_unwrap_parenthesized(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    mode: ParenthesizedUnwrapMode,
) -> bool {
    match mode {
        ParenthesizedUnwrapMode::MemberObject => should_unwrap_parenthesized_member_object(
            context,
            parenthesized_id,
            inner_expression_id,
        ),
        ParenthesizedUnwrapMode::NewMemberCallee => should_unwrap_parenthesized_new_member_callee(
            context,
            parenthesized_id,
            inner_expression_id,
        ),
    }
}

/// Decide whether a parenthesized expression should drop wrappers for a mode.
pub(crate) fn should_drop_parenthesized(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    mode: ParenthesizedDropMode,
) -> bool {
    match mode {
        ParenthesizedDropMode::ExpressionWrapper => should_drop_parenthesized_expression_wrapper(
            context,
            parenthesized_id,
            inner_expression_id,
        ),
        ParenthesizedDropMode::TypeBinaryLeft { node_id } => {
            should_drop_type_binary_left_parentheses(
                context,
                node_id,
                parenthesized_id,
                inner_expression_id,
            )
        }
    }
}

/// Collect postfix star comments from an inner expression that should render after `)`.
pub(crate) fn parenthesized_boundary_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Comment>> {
    let _timing =
        context.timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_BOUNDARY_COMMENTS);
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::TreeExpression { .. }
    ) {
        return Vec::new();
    }

    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return Vec::new();
    }

    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);
    if parenthesized_span.file != inner_span.file || inner_span.end >= parenthesized_span.end {
        return Vec::new();
    }

    let boundary_span = Span::new(
        parenthesized_span.file,
        inner_span.end,
        parenthesized_span.end,
    );
    if !context.has_comment(boundary_span) {
        return Vec::new();
    }

    let comment_trivia = context.tree.comment_trivia();
    let first_relevant_index =
        comment_trivia.partition_point(|comment_trivia| comment_trivia.span.end < inner_span.end);

    let mut comments: Vec<(u32, LocalNodeId<Comment>)> = Vec::new();
    for comment_trivia in comment_trivia[first_relevant_index..].iter().copied() {
        if comment_trivia.span.start > parenthesized_span.end {
            break;
        }
        if context.tree.get(comment_trivia.comment).style != CommentStyle::Star {
            continue;
        }
        if comment_trivia.span.start < inner_span.end
            || comment_trivia.span.end > parenthesized_span.end
        {
            continue;
        }

        if next_non_whitespace_after_span(context, comment_trivia.span) != Some(')') {
            continue;
        }

        comments.push((comment_trivia.span.start, comment_trivia.comment));
    }

    comments.sort_by_key(|(start, _)| *start);
    comments
        .into_iter()
        .map(|(_, comment_id)| comment_id)
        .collect()
}

/// Return whether source contains leading trivia between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_trivia(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    parenthesized_has_leading_inner_pattern(context, parenthesized_id, inner_expression_id, true)
}

/// Return whether source contains leading comments between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    parenthesized_has_leading_inner_pattern(context, parenthesized_id, inner_expression_id, false)
}

/// Return whether source contains a newline between `(` and the inner expression.
pub(crate) fn parenthesized_has_leading_inner_newline(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    context.has_newline(leading_span)
}

/// Return whether source contains leading comment or newline trivia between `(` and inner.
fn parenthesized_has_leading_inner_pattern(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    include_newline: bool,
) -> bool {
    let _timing =
        context.timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_LEADING_TRIVIA);
    let parenthesized_span = context.span(parenthesized_id);
    let inner_span = context.span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    if include_newline && context.has_newline(leading_span) {
        return true;
    }

    context.has_comment(leading_span)
}

fn member_expression_has_optional_chain(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        match context.tree.get(current_id) {
            Expression::Maybe { .. } => return true,
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Call { left, .. }
            | Expression::Must { left, .. }
            | Expression::Instantiation { left, .. } => current_id = *left,
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                current_id = *expression;
            }
            _ => return false,
        }
    }
}

/// Decide whether a parenthesized expression can be unwrapped in member object position.
pub(crate) fn should_unwrap_parenthesized_member_object(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    // object members require explicit grouping: `({}).x`
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::ObjectExpression { .. }
    ) {
        return false;
    }

    // keep nested grouping in type contexts stable across repeated formatting
    if is_type_context(context, parenthesized_id) {
        return false;
    }

    // decorated class expressions require explicit grouping before member access
    if expression_is_decorated_class_declaration(context, inner_expression_id) {
        return false;
    }

    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        // allow unwrapping only when inner annotations are prefix comments or docs
        if !expression_has_only_prefix_comment_or_doc_annotations(context, inner_expression_id) {
            return false;
        }
    }

    if parenthesized_has_leading_inner_comments(context, parenthesized_id, inner_expression_id)
        && !expression_has_only_prefix_comment_or_doc_annotations(context, inner_expression_id)
    {
        return false;
    }

    // optional chains require explicit grouping in non optional member continuations
    if member_expression_has_optional_chain(context, inner_expression_id) {
        return false;
    }

    !needs_parens_in_postfix_position(context.tree, inner_expression_id)
}

/// Return whether a member object should keep parentheses as a `new` callee.
pub(crate) fn member_object_prefers_new_callee_parentheses(
    context: &DestackFormatContext<'_>,
    object_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = object_id;

    while let Expression::Parenthesized { expression } = context.tree.get(current_id) {
        if context.has_annotation(current_id)
            || parenthesized_has_leading_inner_trivia(context, current_id, *expression)
        {
            return false;
        }
        current_id = *expression;
    }

    matches!(
        context.tree.get(current_id),
        Expression::Call { .. } | Expression::Instantiation { .. }
    )
}

/// Return whether a member object is simple enough for `new a.b()` style callee formatting.
pub(crate) fn is_simple_new_member_object(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Path { .. }
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            is_simple_new_member_object(tree, *left)
        }
        Expression::Parenthesized { expression } => is_simple_new_member_object(tree, *expression),
        _ => false,
    }
}

/// Decide whether `new (<member>)()` can unwrap outer parentheses.
pub(crate) fn should_unwrap_parenthesized_new_member_callee(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    match context.tree.get(inner_expression_id) {
        Expression::Path { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            if member_expression_has_optional_chain(context, inner_expression_id) {
                return false;
            }

            is_simple_new_member_object(context.tree, *left)
        }
        _ => false,
    }
}

/// Return whether a declaration expression is a decorated class declaration.
pub(crate) fn expression_is_decorated_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Class { .. } = context.tree.get(*declaration_id) else {
        return false;
    };

    let expression_has_decorator = context.visit_annotations(expression_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id),
                Annotation::Decorator { .. }
            )
        })
    });
    if expression_has_decorator.unwrap_or(false) {
        return true;
    }

    context
        .visit_annotations(*declaration_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Decorator { .. }
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether a parenthesized expression wraps a decorated class in `extends`.
pub(crate) fn parenthesized_wraps_decorated_class_extends_head(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let extends_types = match context.tree.get(declaration_id) {
        Declaration::Class { heritage, .. } => heritage.extends_types.as_deref(),
        _ => None,
    };
    let Some(extends_types) = extends_types else {
        return false;
    };

    extends_types.contains(&node_id)
        && expression_is_decorated_class_declaration(context, inner_expression_id)
}

/// Return whether a parenthesized extends head carries prefix comment/doc annotations.
pub(crate) fn parenthesized_wraps_prefix_annotated_class_extends_head(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let extends_types = match context.tree.get(declaration_id) {
        Declaration::Class { heritage, .. } => heritage.extends_types.as_deref(),
        _ => None,
    };
    let Some(extends_types) = extends_types else {
        return false;
    };
    if !extends_types.contains(&node_id) {
        return false;
    }

    expression_has_prefix_comment_or_doc_annotation_in_left_spine(context, inner_expression_id)
}

/// Return whether annotations are only prefix comment/doc markers for this expression.
fn expression_has_only_prefix_comment_or_doc_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotations| {
            !annotations.is_empty()
                && annotations.iter().all(|annotation_id| {
                    matches!(
                        context.annotation(*annotation_id),
                        Annotation::Comment {
                            position: AnnotationPosition::LinePrefix
                                | AnnotationPosition::BlockPrefix,
                            ..
                        } | Annotation::Doc {
                            position: AnnotationPosition::LinePrefix
                                | AnnotationPosition::BlockPrefix,
                            ..
                        }
                    )
                })
        })
        .unwrap_or(false)
}

/// Return whether any expression on the left spine has a prefix comment/doc annotation.
pub(crate) fn expression_has_prefix_comment_or_doc_annotation_in_left_spine(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        if expression_has_only_prefix_comment_or_doc_annotations(context, current_id) {
            return true;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } => Some(*expression),
            Expression::Call { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            | Expression::TypeBinary { left, .. }
            | Expression::Binary { left, .. } => Some(*left),
            _ => None,
        };

        let Some(next_id) = next_id else {
            break;
        };
        current_id = next_id;
    }

    false
}

/// Keep parentheses for cast or satisfies expressions in statement position.
pub(crate) fn type_binary_is_statement_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    matches!(
        context.tree.get(parent_id),
        Expression::Statement(inner) if *inner == node_id
    )
}

/// Return whether a type-binary expression is wrapped by one statement parenthesized node.
pub(crate) fn type_binary_is_parenthesized_statement_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = context.tree.get(parent_id) else {
        return false;
    };
    if *expression != node_id {
        return false;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_id = LocalNodeId::<Expression>::new(grandparent_id);
    matches!(
        context.tree.get(grandparent_id),
        Expression::Statement(inner_id) if *inner_id == parent_id
    )
}

/// Return whether any parenthesized expression ancestor has leading inner trivia.
pub(crate) fn has_parenthesized_ancestor_with_leading_inner_trivia(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;

    while let Some((parent_id, parent_type)) = context.parent(current_id) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        if let Expression::Parenthesized { expression } = context.tree.get(parent_id)
            && *expression == current_id
            && parenthesized_has_leading_inner_trivia(context, parent_id, current_id)
        {
            return true;
        }

        current_id = parent_id;
    }

    false
}

/// Return whether cast or satisfies appears as the parenthesized callee of a `new` expression.
pub(crate) fn type_binary_is_parenthesized_new_callee(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = context.tree.get(parent_id) else {
        return false;
    };
    if *expression != node_id {
        return false;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_id = LocalNodeId::<Expression>::new(grandparent_id);
    matches!(
        context.tree.get(grandparent_id),
        Expression::New { left, .. } if *left == parent_id
    )
}

/// Decide whether cast or satisfies can drop a parenthesized left side.
pub(crate) fn should_drop_type_binary_left_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parenthesized_id: LocalNodeId<Expression>,
    left_id: LocalNodeId<Expression>,
) -> bool {
    let left_is_cast_chain = matches!(
        context.tree.get(left_id),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    );

    if context.has_annotation(parenthesized_id) {
        return false;
    }

    if context.has_annotation(left_id) && !left_is_cast_chain {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, left_id)
        && !left_is_cast_chain
    {
        return false;
    }

    if type_binary_is_statement_expression(context, node_id)
        || type_binary_is_parenthesized_statement_expression(context, node_id)
    {
        return false;
    }

    if has_parenthesized_ancestor_with_leading_inner_trivia(context, node_id) && !left_is_cast_chain
    {
        return false;
    }

    is_simple_type_binary_left_expression(context.tree, left_id)
}

/// Return whether expression annotations are only prefix comments before `|` or `&`.
fn expression_has_only_type_grouping_prefix_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return false;
    };
    if annotation_ids.is_empty() {
        return false;
    }

    annotation_ids.into_iter().all(|annotation_id| {
        let annotation = context.annotation(annotation_id);
        let Annotation::Comment {
            position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
            ..
        } = annotation
        else {
            return false;
        };

        let annotation_span = context.annotation_span(annotation_id);
        matches!(
            next_non_whitespace_after_span(context, annotation_span),
            Some('/' | '|' | '&')
        )
    })
}

/// Store signals for parenthesized type-wrapper drop mode.
#[derive(Debug, Clone, Copy)]
struct ParenthesizedTypeDrop {
    /// Whether the wrapper node has annotations.
    has_wrapper_annotation: bool,
    /// Whether inner annotations include non-grouping cases.
    has_non_grouping_inner_annotation: bool,
    /// Whether wrapper is one decorated class extends head.
    wraps_decorated_class_extends_head: bool,
    /// Whether wrapper runs in a type context.
    is_type_context: bool,
    /// Whether wrapper can drop as array element type wrapper.
    can_drop_array_element_wrapper: bool,
    /// Whether wrapper can drop as associative type binary wrapper.
    can_drop_associative_binary_wrapper: bool,
    /// Whether wrapper can drop as a conditional type arm inside `|` or `&`.
    can_drop_conditional_type_grouping_wrapper: bool,
    /// Whether wrapper is the sole operand in a type-grouping chain.
    can_drop_single_operand_type_grouping_wrapper: bool,
    /// Whether wrapper drop is safe in parent context.
    is_safe_in_parent_context: bool,
    /// Whether wrapper parent is an expression node.
    parent_is_expression: bool,
    /// Whether wrapper is the declared return type of one function signature.
    parent_is_function_return_type: bool,
    /// Whether wrapper is one conditional type arm (`extends` left or right).
    parent_is_type_conditional_arm: bool,
    /// Whether inner expression is one lambda declaration type.
    inner_is_lambda_declaration: bool,
    /// Whether inner expression is one conditional type.
    inner_is_type_conditional: bool,
    /// Whether inner expression is one simple type-binary left chain.
    inner_is_simple_type_binary_left: bool,
}

/// Collect parenthesized type-wrapper drop signals.
fn parenthesized_type_drop(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> ParenthesizedTypeDrop {
    // annotation signals
    let has_wrapper_annotation = context.has_annotation(node_id);
    let has_non_grouping_inner_annotation = context.has_annotation(inner_id)
        && !expression_has_only_type_grouping_prefix_annotations(context, inner_id);

    // context signals
    let wraps_decorated_class_extends_head =
        parenthesized_wraps_decorated_class_extends_head(context, node_id, inner_id);
    let is_type_context = is_type_context(context, node_id);
    // early-drop signals
    let can_drop_array_element_wrapper =
        context
            .parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                if parent_type != NodeType::Expression {
                    return false;
                }

                let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                if let Expression::Index { left, index, .. } =
                    context.tree.get(parent_expression_id)
                {
                    *left == node_id
                        && index.is_none()
                        && !has_wrapper_annotation
                        && !context.has_annotation(inner_id)
                        && is_simple_type_binary_left_expression(context.tree, inner_id)
                } else {
                    false
                }
            });
    let can_drop_associative_binary_wrapper =
        parenthesized_associative_type_binary_can_drop(context, node_id, inner_id);
    let can_drop_conditional_type_grouping_wrapper =
        parenthesized_conditional_type_grouping_can_drop(context, node_id, inner_id);
    let can_drop_single_operand_type_grouping_wrapper =
        parenthesized_single_operand_type_grouping_can_drop(context, node_id);
    // grouping signals
    let is_safe_in_parent_context =
        parenthesized_type_grouping_drop_is_safe_in_parent(context, node_id);
    let parent = context.parent(node_id);
    let parent_is_expression =
        parent.is_some_and(|(_, parent_type)| parent_type == NodeType::Expression);
    let parent_is_function_return_type = parent.is_some_and(|(parent_id, parent_type)| {
        if parent_type != NodeType::Declaration {
            return false;
        }

        let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
        matches!(
            context.tree.get(declaration_id),
            Declaration::Function { signature, .. } if signature.return_type == Some(node_id)
        )
    });
    let parent_is_type_conditional_arm = parent.is_some_and(|(parent_id, parent_type)| {
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        matches!(
            context.tree.get(parent_expression_id),
            Expression::TypeConditional { left, right, .. }
                if *left == node_id || *right == node_id
        )
    });

    // base shape signals
    let inner_is_lambda_declaration = matches!(
        context.tree.get(inner_id),
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
            )
    );
    let inner_is_type_conditional = matches!(
        context.tree.get(inner_id),
        Expression::TypeConditional { .. }
    );
    let inner_is_simple_type_binary_left =
        is_simple_type_binary_left_expression(context.tree, inner_id);

    ParenthesizedTypeDrop {
        has_wrapper_annotation,
        has_non_grouping_inner_annotation,
        wraps_decorated_class_extends_head,
        is_type_context,
        can_drop_array_element_wrapper,
        can_drop_associative_binary_wrapper,
        can_drop_conditional_type_grouping_wrapper,
        can_drop_single_operand_type_grouping_wrapper,
        is_safe_in_parent_context,
        parent_is_expression,
        parent_is_function_return_type,
        parent_is_type_conditional_arm,
        inner_is_lambda_declaration,
        inner_is_type_conditional,
        inner_is_simple_type_binary_left,
    }
}

/// Decide type-wrapper drop rules from collected signals.
fn should_drop_parenthesized_type(signals: ParenthesizedTypeDrop) -> bool {
    // wrapper annotations are semantic boundaries and should keep grouping
    if signals.has_wrapper_annotation {
        return false;
    }

    // inner annotations are usually semantic boundaries
    // but leading type-grouping prefix comments are compatible with dropping the wrapper
    if signals.has_non_grouping_inner_annotation {
        return false;
    }

    // decorated class extends heads must preserve explicit grouping
    if signals.wraps_decorated_class_extends_head {
        return false;
    }

    if !signals.is_type_context {
        return false;
    }

    // drop redundant simple wrappers in array element position: `(number)[]` -> `number[]`
    if signals.can_drop_array_element_wrapper {
        return true;
    }

    if signals.can_drop_associative_binary_wrapper {
        return true;
    }

    if signals.can_drop_conditional_type_grouping_wrapper {
        return true;
    }

    if signals.can_drop_single_operand_type_grouping_wrapper {
        return true;
    }

    if !signals.is_safe_in_parent_context {
        return false;
    }

    if !signals.parent_is_expression && signals.inner_is_type_conditional {
        return true;
    }

    if signals.inner_is_lambda_declaration
        && !signals.parent_is_function_return_type
        && !signals.parent_is_type_conditional_arm
    {
        return true;
    }

    signals.inner_is_simple_type_binary_left
}

/// Return whether a wrapper is the only operand in a type `|` or `&` grouping chain.
fn parenthesized_single_operand_type_grouping_can_drop(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Binary { operator, .. } = context.tree.get(parent_expression_id) else {
        return false;
    };
    if !matches!(
        operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    ) || !is_type_context(context, parent_expression_id)
    {
        return false;
    }

    let operands = flatten_type_binary_expression(context, parent_expression_id, *operator);
    operands.len() == 1
}

/// Return whether a conditional type wrapper is redundant in `|` or `&` grouping chains.
fn parenthesized_conditional_type_grouping_can_drop(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    if !matches!(
        context.tree.get(inner_id),
        Expression::TypeConditional { .. }
    ) {
        return false;
    }

    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_expression_id) {
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            (*left == node_id || *right == node_id)
                && matches!(
                    operator,
                    BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                )
                && is_type_context(context, parent_expression_id)
        }
        _ => false,
    }
}

/// Decide whether a parenthesized type expression can drop wrappers.
pub(crate) fn should_drop_parenthesized_type_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let _timing = context.timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_TYPE_DROP);
    let signals = parenthesized_type_drop(context, node_id, inner_id);
    should_drop_parenthesized_type(signals)
}

/// Return whether dropping a parenthesized type grouping is safe in the parent expression context.
fn parenthesized_type_grouping_drop_is_safe_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return true;
    };
    if parent_type != NodeType::Expression {
        return true;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_expression_id) {
        Expression::Index { left, .. } | Expression::TypeIndex { left, .. } => *left != node_id,
        _ => true,
    }
}

/// Return whether a binary operator is associative in type contexts.
pub(crate) fn is_associative_type_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    )
}

/// Decide whether an associative type binary can drop redundant wrappers.
pub(crate) fn parenthesized_associative_type_binary_can_drop(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Binary {
        operator: inner_operator,
        ..
    } = context.tree.get(inner_id)
    else {
        return false;
    };
    if !is_associative_type_binary_operator(*inner_operator) {
        return false;
    }

    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            (*left == node_id || *right == node_id)
                && *operator == *inner_operator
                && is_type_context(context, parent_id)
        }
        _ => false,
    }
}

/// Return whether a parenthesized call callee wrapper can drop safely.
fn parenthesized_call_callee_wrapper_can_drop(
    context: &DestackFormatContext<'_>,
    parent_expression: &Expression,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = parent_expression else {
        return false;
    };
    if *left != node_id {
        return false;
    }

    if !expression_has_trailing_static_instantiation(context.tree, inner_expression_id) {
        return false;
    }

    if context.has_annotation(node_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    !parenthesized_has_leading_inner_trivia(context, node_id, inner_expression_id)
}

/// Return whether one expression ends in static instantiation arguments.
fn expression_has_trailing_static_instantiation(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        Expression::Path {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Parenthesized { expression } => {
            expression_has_trailing_static_instantiation(tree, *expression)
        }
        _ => false,
    }
}

/// Return whether a tree expression contains ternary branches wrapped in parentheses.
fn tree_expression_has_parenthesized_ternary_branch(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::TreeExpression { elements, .. } = tree.get(expression_id) else {
        return false;
    };
    let Some(elements) = elements.as_ref() else {
        return false;
    };

    elements.iter().any(|argument_id| {
        let argument = tree.get(*argument_id);
        let value_id = match argument {
            Argument::Positional { value, .. }
            | Argument::Spread { value, .. }
            | Argument::Named { value, .. }
            | Argument::Labeled { value, .. } => *value,
        };

        let Expression::If {
            kind: IfKind::Ternary,
            then_expression,
            else_expression,
            ..
        } = tree.get(value_id)
        else {
            return false;
        };

        matches!(tree.get(*then_expression), Expression::Parenthesized { .. })
            || else_expression.is_some_and(|else_id| {
                matches!(tree.get(else_id), Expression::Parenthesized { .. })
            })
    })
}

/// Store signals for generic parenthesized-wrapper drop mode.
#[derive(Debug, Clone, Copy)]
struct ParenthesizedExpressionDrop {
    /// Whether wrapper is one decorated class extends head.
    wraps_decorated_class_extends_head: bool,
    /// Whether wrapper is one prefix-annotated class extends head.
    wraps_prefix_annotated_class_extends_head: bool,
    /// Whether type-wrapper drop mode allows dropping.
    should_drop_type_parentheses: bool,
    /// Whether wrapper has any parent.
    has_parent: bool,
    /// Whether wrapper parent is one expression.
    parent_is_expression: bool,
    /// Whether argument parent allows decorated class wrapper drop.
    should_drop_argument_decorated_class_wrapper: bool,
    /// Whether declarator parent allows tree wrapper drop.
    should_drop_declarator_tree_wrapper: bool,
    /// Whether declarator parent allows prefix wrapper drop.
    should_drop_declarator_prefix_wrapper: bool,
    /// Whether expression parent allows statement type-binary wrapper drop.
    should_drop_statement_type_binary_wrapper: bool,
    /// Whether expression parent allows assignment-must wrapper drop.
    should_drop_assignment_must: bool,
    /// Whether expression parent allows statement lambda wrapper drop.
    should_drop_statement_lambda: bool,
    /// Whether expression parent allows call-callee instantiation wrapper drop.
    should_drop_call_callee_instantiation_wrapper: bool,
    /// Whether one optional-chain wrapper must stay grouped in postfix parent contexts.
    should_keep_optional_chain_postfix_wrapper: bool,
}

/// Return whether one parenthesized optional-chain wrapper must stay grouped in a postfix parent.
fn should_keep_optional_chain_postfix_wrapper(
    context: &DestackFormatContext<'_>,
    parent_expression: &Expression,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if !member_expression_has_optional_chain(context, inner_expression_id) {
        return false;
    }

    match parent_expression {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Must { left, .. }
        | Expression::New { left, .. } => *left == node_id,
        Expression::Call { left, position, .. } | Expression::Index { left, position, .. } => {
            *left == node_id && *position == PostfixPosition::Direct
        }
        _ => false,
    }
}

/// Collect generic parenthesized-wrapper drop signals.
fn parenthesized_expression_drop(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> ParenthesizedExpressionDrop {
    // wrapper and type-drop signals
    let wraps_decorated_class_extends_head =
        parenthesized_wraps_decorated_class_extends_head(context, node_id, inner_expression_id);
    let wraps_prefix_annotated_class_extends_head =
        parenthesized_wraps_prefix_annotated_class_extends_head(
            context,
            node_id,
            inner_expression_id,
        );
    let should_drop_type_parentheses =
        should_drop_parenthesized_type_expression(context, node_id, inner_expression_id);

    // parent signals
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return ParenthesizedExpressionDrop {
            wraps_decorated_class_extends_head,
            wraps_prefix_annotated_class_extends_head,
            should_drop_type_parentheses,
            has_parent: false,
            parent_is_expression: false,
            should_drop_argument_decorated_class_wrapper: false,
            should_drop_declarator_tree_wrapper: false,
            should_drop_declarator_prefix_wrapper: false,
            should_drop_statement_type_binary_wrapper: false,
            should_drop_assignment_must: false,
            should_drop_statement_lambda: false,
            should_drop_call_callee_instantiation_wrapper: false,
            should_keep_optional_chain_postfix_wrapper: false,
        };
    };

    if parent_type != NodeType::Expression {
        let should_drop_argument_decorated_class_wrapper = parent_type == NodeType::Argument
            && !context.has_annotation(node_id)
            && !parenthesized_has_leading_inner_newline(context, node_id, inner_expression_id)
            && expression_is_decorated_class_declaration(context, inner_expression_id);
        let should_drop_declarator_tree_wrapper = parent_type == NodeType::Declarator
            && !context.has_annotation(node_id)
            && !parenthesized_has_leading_inner_newline(context, node_id, inner_expression_id)
            && !context.node_has_newline(inner_expression_id)
            && !tree_expression_has_parenthesized_ternary_branch(context.tree, inner_expression_id)
            && matches!(
                context.tree.get(inner_expression_id),
                Expression::TreeExpression {
                    arguments,
                    elements,
                    ..
                } if !tree_literal_should_expand(context, arguments, elements)
            );
        let should_drop_declarator_prefix_wrapper = parent_type == NodeType::Declarator
            && !context.has_annotation(node_id)
            && !parenthesized_has_leading_inner_newline(context, node_id, inner_expression_id)
            && expression_has_prefix_comment_or_doc_annotation_in_left_spine(
                context,
                inner_expression_id,
            );

        return ParenthesizedExpressionDrop {
            wraps_decorated_class_extends_head,
            wraps_prefix_annotated_class_extends_head,
            should_drop_type_parentheses,
            has_parent: true,
            parent_is_expression: false,
            should_drop_argument_decorated_class_wrapper,
            should_drop_declarator_tree_wrapper,
            should_drop_declarator_prefix_wrapper,
            should_drop_statement_type_binary_wrapper: false,
            should_drop_assignment_must: false,
            should_drop_statement_lambda: false,
            should_drop_call_callee_instantiation_wrapper: false,
            should_keep_optional_chain_postfix_wrapper: false,
        };
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_id);
    let inner_expression = context.tree.get(inner_expression_id);
    let should_drop_statement_type_binary_wrapper = matches!(
        parent_expression,
        Expression::Statement(inner_id) if inner_id.id == node_id.id
    ) && matches!(
        inner_expression,
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    ) && !context.has_annotation(node_id);
    let should_drop_assignment_must = matches!(
        parent_expression,
        Expression::Assign { left, .. } if *left == node_id
    ) && matches!(inner_expression, Expression::Must { .. });
    let should_drop_statement_lambda = matches!(
        parent_expression,
        Expression::Statement(inner_id) if inner_id.id == node_id.id
    ) && matches!(
        inner_expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
            )
    ) && !context.has_annotation(node_id);
    let should_drop_call_callee_instantiation_wrapper = parenthesized_call_callee_wrapper_can_drop(
        context,
        parent_expression,
        node_id,
        inner_expression_id,
    );
    let should_keep_optional_chain_postfix_wrapper = should_keep_optional_chain_postfix_wrapper(
        context,
        parent_expression,
        node_id,
        inner_expression_id,
    );

    ParenthesizedExpressionDrop {
        wraps_decorated_class_extends_head,
        wraps_prefix_annotated_class_extends_head,
        should_drop_type_parentheses,
        has_parent: true,
        parent_is_expression: true,
        should_drop_argument_decorated_class_wrapper: false,
        should_drop_declarator_tree_wrapper: false,
        should_drop_declarator_prefix_wrapper: false,
        should_drop_statement_type_binary_wrapper,
        should_drop_assignment_must,
        should_drop_statement_lambda,
        should_drop_call_callee_instantiation_wrapper,
        should_keep_optional_chain_postfix_wrapper,
    }
}

/// Decide generic parenthesized-wrapper drop mode from collected signals.
fn should_drop_parenthesized_expression(signals: ParenthesizedExpressionDrop) -> bool {
    // decorated class extends heads must keep explicit grouping
    if signals.wraps_decorated_class_extends_head {
        return false;
    }

    // closure-style cast wrappers in class heritage should stay explicit
    if signals.wraps_prefix_annotated_class_extends_head {
        return false;
    }

    // optional chain wrappers preserve non optional continuation semantics
    if signals.should_keep_optional_chain_postfix_wrapper {
        return false;
    }

    if !signals.has_parent {
        return signals.should_drop_type_parentheses;
    }

    if !signals.parent_is_expression {
        if signals.should_drop_argument_decorated_class_wrapper
            || signals.should_drop_declarator_tree_wrapper
            || signals.should_drop_declarator_prefix_wrapper
        {
            return true;
        }

        return signals.should_drop_type_parentheses;
    }

    signals.should_drop_statement_type_binary_wrapper
        || signals.should_drop_assignment_must
        || signals.should_drop_statement_lambda
        || signals.should_drop_call_callee_instantiation_wrapper
        || signals.should_drop_type_parentheses
}

/// Decide whether a parenthesized expression should drop wrappers in generic expression contexts.
pub(crate) fn should_drop_parenthesized_expression_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let _timing = context.timing_scope(timing::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_DROP_MODE);
    let signals = parenthesized_expression_drop(context, node_id, inner_expression_id);
    should_drop_parenthesized_expression(signals)
}
