use crate::format::annotation::{
    format_raw_comment, infix_or_postfix_annotations, prefix_annotations,
    write_inline_prefix_annotations,
};
use crate::format::chain::{is_chain_root, is_expression_chain, transparent_inner_expression};
use crate::format::context::ParenthesizedExpressionView;
use crate::format::declaration::is_poorly_breakable_member_or_call_chain;
use crate::format::expression::{
    expression_has_prefix_comment_or_doc_annotation_in_left_spine,
    expression_has_static_type_arguments, is_expression_breakable,
    write_expression_without_prefix_annotations,
};
use crate::format::operator::{
    AssignmentLikeLayout, assignment_rhs_prefers_break_after_operator, write_assignment_like_right,
    write_type_expression_with_inline_prefix_annotations,
};
use crate::format::tree::tree_literal_requires_expanded_layout;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Argument, Comment, Declaration, Declarator, Expression, FunctionKind,
    IfKind, LocalNodeId, NodeTree, Pattern, PatternField, ScalarLiteral, TokenType,
};
use destack_fir::format::{
    Buffer, FormatNode as FirFormatNode, FormatNodes, FormatResult, Formatter as FirFormatter,
    VecBuffer,
};
use destack_fir::prelude::{format_with, group, hard_line_break, space, token};
use destack_fir::write;
use destack_source::Span;

const MIN_OVERLAP_FOR_BREAK: u32 = 3;

/// Return whether one expression has an own-line prefix annotation.
fn declarator_expression_has_own_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .annotation_ids(expression_id)
        .iter()
        .copied()
        .any(|annotation_id| {
            let annotation = context.annotation(annotation_id);
            if !matches!(
                annotation.position(),
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ) {
                return false;
            }

            context.annotation_starts_on_own_line(annotation_id)
        })
}

/// Buffer the declarator header so layout can inspect the formatted lhs first.
fn buffer_declarator_header<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern_id: LocalNodeId<Pattern>,
    type_id: Option<LocalNodeId<Expression>>,
) -> FormatResult<(Vec<FirFormatNode>, bool, bool)> {
    let mut buffer = VecBuffer::new(f.state_mut());
    let formatter = &mut FirFormatter::new(&mut buffer);

    // pattern
    write!(formatter, [pattern_id])?;

    // type annotation
    if let Some(type_id) = type_id {
        write!(formatter, [token(":"), space()])?;
        write_type_expression_with_inline_prefix_annotations(formatter, type_id)?;
    }

    let nodes = buffer.into_vec();
    let is_short = nodes.single_line_width().is_some_and(|width| {
        width < (u32::from(f.context().options.indent_width) + MIN_OVERLAP_FOR_BREAK)
    });
    let may_break = nodes.may_directly_break();

    Ok((nodes, is_short, may_break))
}

/// Return whether one rhs is lambda-like for assignment-like layout.
fn declarator_value_is_lambda_like(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => matches!(
            context.tree.get(*declaration_id),
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        ),
        _ => false,
    }
}

/// Return whether one pattern subtree contains at least one default assignment.
pub(crate) fn pattern_has_default_assignment(
    tree: &NodeTree,
    pattern_id: LocalNodeId<Pattern>,
) -> bool {
    match tree.get(pattern_id) {
        Pattern::Wildcard | Pattern::Expression { .. } => false,
        Pattern::Must(inner_pattern_id)
        | Pattern::ReferenceOf {
            right: inner_pattern_id,
            ..
        }
        | Pattern::ValueOf {
            right: inner_pattern_id,
            ..
        } => pattern_has_default_assignment(tree, *inner_pattern_id),
        Pattern::Binding { pattern, .. } => pattern.as_ref().is_some_and(|inner_pattern_id| {
            pattern_has_default_assignment(tree, *inner_pattern_id)
        }),
        Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. }
        | Pattern::Array { fields }
        | Pattern::Object { fields }
        | Pattern::TaggedObject { fields, .. } => fields
            .iter()
            .copied()
            .any(|field_id| pattern_field_has_default_assignment(tree, field_id)),
        Pattern::Union { patterns } => patterns
            .iter()
            .copied()
            .any(|inner_pattern_id| pattern_has_default_assignment(tree, inner_pattern_id)),
    }
}

/// Return whether one pattern field contains at least one default assignment.
fn pattern_field_has_default_assignment(
    tree: &NodeTree,
    pattern_field_id: LocalNodeId<PatternField>,
) -> bool {
    match tree.get(pattern_field_id) {
        PatternField::Named {
            pattern, default, ..
        }
        | PatternField::Computed {
            pattern, default, ..
        } => {
            default.is_some()
                || pattern
                    .as_ref()
                    .is_some_and(|pattern_id| pattern_has_default_assignment(tree, *pattern_id))
        }
        PatternField::Alias { default, .. } => default.is_some(),
        PatternField::Positional { pattern, default } => {
            default.is_some() || pattern_has_default_assignment(tree, *pattern)
        }
        PatternField::Spread { pattern, .. } => pattern
            .as_ref()
            .is_some_and(|pattern_id| pattern_has_default_assignment(tree, *pattern_id)),
        PatternField::Elision => false,
    }
}

/// Return whether one pattern subtree contains one default assignment below top-level fields.
fn pattern_has_nested_default_assignment(
    tree: &NodeTree,
    pattern_id: LocalNodeId<Pattern>,
) -> bool {
    pattern_has_nested_default_assignment_at_depth(tree, pattern_id, 0)
}

/// Return whether one pattern subtree contains one default assignment at depth greater than zero.
fn pattern_has_nested_default_assignment_at_depth(
    tree: &NodeTree,
    pattern_id: LocalNodeId<Pattern>,
    depth: usize,
) -> bool {
    match tree.get(pattern_id) {
        Pattern::Wildcard | Pattern::Expression { .. } => false,
        Pattern::Must(inner_pattern_id)
        | Pattern::ReferenceOf {
            right: inner_pattern_id,
            ..
        }
        | Pattern::ValueOf {
            right: inner_pattern_id,
            ..
        } => pattern_has_nested_default_assignment_at_depth(tree, *inner_pattern_id, depth),
        Pattern::Binding { pattern, .. } => pattern.as_ref().is_some_and(|inner_pattern_id| {
            pattern_has_nested_default_assignment_at_depth(tree, *inner_pattern_id, depth)
        }),
        Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. }
        | Pattern::Array { fields }
        | Pattern::Object { fields }
        | Pattern::TaggedObject { fields, .. } => fields
            .iter()
            .copied()
            .any(|field_id| pattern_field_has_nested_default_assignment(tree, field_id, depth + 1)),
        Pattern::Union { patterns } => patterns.iter().copied().any(|inner_pattern_id| {
            pattern_has_nested_default_assignment_at_depth(tree, inner_pattern_id, depth)
        }),
    }
}

/// Return whether one pattern field contains one nested default assignment.
fn pattern_field_has_nested_default_assignment(
    tree: &NodeTree,
    pattern_field_id: LocalNodeId<PatternField>,
    depth: usize,
) -> bool {
    match tree.get(pattern_field_id) {
        PatternField::Named {
            pattern, default, ..
        }
        | PatternField::Computed {
            pattern, default, ..
        } => {
            (depth > 1 && default.is_some())
                || pattern.as_ref().is_some_and(|pattern_id| {
                    pattern_has_nested_default_assignment_at_depth(tree, *pattern_id, depth)
                })
        }
        PatternField::Alias { default, .. } => depth > 1 && default.is_some(),
        PatternField::Positional { pattern, default } => {
            (depth > 1 && default.is_some())
                || pattern_has_nested_default_assignment_at_depth(tree, *pattern, depth)
        }
        PatternField::Spread { pattern, .. } => pattern.as_ref().is_some_and(|pattern_id| {
            pattern_has_nested_default_assignment_at_depth(tree, *pattern_id, depth)
        }),
        PatternField::Elision => false,
    }
}

/// Return whether one pattern is array-like after transparent wrapper unwrapping.
fn pattern_is_array_like(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
    match tree.get(pattern_id) {
        Pattern::Binding { pattern, .. } => pattern
            .as_ref()
            .is_some_and(|inner_pattern_id| pattern_is_array_like(tree, *inner_pattern_id)),
        Pattern::Must(inner_pattern_id)
        | Pattern::ReferenceOf {
            right: inner_pattern_id,
            ..
        }
        | Pattern::ValueOf {
            right: inner_pattern_id,
            ..
        } => pattern_is_array_like(tree, *inner_pattern_id),
        Pattern::Array { .. } | Pattern::Tuple { .. } | Pattern::TaggedTuple { .. } => true,
        _ => false,
    }
}

/// Return whether one pattern is complex enough to break before `=`.
fn pattern_is_complex_destructuring(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
    match tree.get(pattern_id) {
        Pattern::Binding { pattern, .. } => pattern.as_ref().is_some_and(|inner_pattern_id| {
            pattern_is_complex_destructuring(tree, *inner_pattern_id)
        }),
        Pattern::Must(inner_pattern_id)
        | Pattern::ReferenceOf {
            right: inner_pattern_id,
            ..
        }
        | Pattern::ValueOf {
            right: inner_pattern_id,
            ..
        } => pattern_is_complex_destructuring(tree, *inner_pattern_id),
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => fields.len() > 2,
        Pattern::Array { fields }
        | Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. } => fields.len() > 2,
        _ => false,
    }
}

/// Return whether one declarator value has one prefix annotation after the `=` operator.
pub(crate) fn declarator_value_has_assignment_operator_prefix_comment(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    context
        .raw_prefix_doc_comments_for(value_id)
        .iter()
        .any(|comment| {
            context
                .previous_non_trivia_token_before_span(comment.span)
                .is_some_and(|token| token.token.ty == TokenType::Assign)
        })
}

/// Return raw comment trivia nodes after the `=` operator for one declarator value.
fn declarator_value_assignment_operator_comment_nodes(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let value_span = context.span(value_id);
    let Some(previous_token) = context.previous_non_trivia_token_before_span(value_span) else {
        return Vec::new();
    };

    if previous_token.token.ty != TokenType::Assign
        || previous_token.span.file != value_span.file
        || previous_token.span.end >= value_span.start
    {
        return Vec::new();
    }

    {
        let comments = context.comments();
        comments
            .comments_in_range(previous_token.span.end, value_span.start)
            .to_vec()
    }
}

/// Write raw comments between one `=` operator and rhs expression.
fn write_assignment_operator_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    omit_leading_separator: bool,
) -> FormatResult<()> {
    let value_span = f.context().span(value_id);
    let Some(previous_token) = f
        .context()
        .previous_non_trivia_token_before_span(value_span)
    else {
        return Ok(());
    };
    if previous_token.token.ty != TokenType::Assign
        || previous_token.span.file != value_span.file
        || previous_token.span.end >= value_span.start
    {
        return Ok(());
    }

    let comment_nodes = {
        let comments = f.context().comments();
        comments
            .comments_in_range(previous_token.span.end, value_span.start)
            .to_vec()
    };
    if comment_nodes.is_empty() {
        return Ok(());
    }

    let first_comment_span = comment_nodes[0].span;
    let leading_gap = Span::new(
        value_span.file,
        previous_token.span.end,
        first_comment_span.start,
    );
    if !omit_leading_separator {
        if f.context().has_newline(leading_gap)
            || f.context().span_starts_on_own_line(first_comment_span)
        {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    for (index, comment) in comment_nodes.iter().copied().enumerate() {
        let comment_span = comment.span;
        format_raw_comment(f, comment)?;

        let is_last = index + 1 == comment_nodes.len();
        if !is_last
            || comment.is_line()
            || f.context()
                .span_has_newline_before_next_non_whitespace_token(comment_span)
        {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Format one declarator rhs while preserving assignment-operator prefix ownership.
fn format_assignment_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    value_has_assignment_operator_prefix_annotation: bool,
    assignment_operator_comment_nodes: &[Comment],
) -> FormatResult<()> {
    let prefix_annotation_ids: Vec<_> = f
        .context()
        .annotation_ids(value_id)
        .iter()
        .copied()
        .filter(|annotation_id| {
            matches!(
                f.context().annotation(*annotation_id).position(),
                AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
            )
        })
        .collect();

    let mut operation = || {
        if prefix_annotation_ids.is_empty() {
            if !assignment_operator_comment_nodes.is_empty() {
                return write_expression_without_prefix_annotations(f, value_id);
            }

            write!(f, [value_id])?;
            return Ok(());
        }

        if value_has_assignment_operator_prefix_annotation {
            write_inline_prefix_annotations(f, &prefix_annotation_ids)?;
            write!(f, [space()])?;
            return write_expression_without_prefix_annotations(f, value_id);
        }

        write!(f, [prefix_annotations(f.context(), value_id)])?;

        if let Some(last_prefix_annotation_id) = prefix_annotation_ids.last().copied()
            && f.context()
                .annotation_next_token_is_on_same_line(last_prefix_annotation_id)
        {
            write!(f, [space()])?;
        }

        write_expression_without_prefix_annotations(f, value_id)
    };

    if assignment_operator_comment_nodes.is_empty() {
        return operation();
    }

    operation()
}

/// Decide whether a declarator can drop one parenthesized value wrapper.
pub(crate) fn declarator_drops_parenthesized_value_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let normalized_inner_expression_id = transparent_inner_expression(context, inner_expression_id);

    if context.has_annotation(parenthesized_id) {
        return false;
    }

    if ParenthesizedExpressionView::from_node(context, parenthesized_id)
        .is_some_and(ParenthesizedExpressionView::has_leading_inner_newline)
    {
        return false;
    }

    // keep multiline tree and nested ternary wrappers explicit in declarators
    let drops_tree_wrapper = !context.node_has_newline(inner_expression_id)
        && !matches!(
            context.tree.get(inner_expression_id),
            Expression::TreeExpression { elements: Some(elements), .. }
                if elements.iter().any(|argument_id| {
                    let value_id = match context.tree.get(*argument_id) {
                        Argument::Positional { value, .. }
                        | Argument::Spread { value, .. }
                        | Argument::Named { value, .. }
                        | Argument::Labeled { value, .. } => *value,
                        Argument::Error => return false,
                    };

                    let Expression::If {
                        kind: IfKind::Ternary,
                        then_expression,
                        else_expression,
                        ..
                    } = context.tree.get(value_id)
                    else {
                        return false;
                    };

                    matches!(context.tree.get(*then_expression), Expression::Parenthesized { .. })
                        || else_expression.is_some_and(|else_id| {
                            matches!(context.tree.get(else_id), Expression::Parenthesized { .. })
                        })
                })
        )
        && matches!(
            context.tree.get(inner_expression_id),
            Expression::TreeExpression {
                arguments,
                elements,
                ..
            } if !tree_literal_requires_expanded_layout(context, arguments, elements)
        );

    // keep left-spine prefix ownership intact when the declarator already owns the boundary
    let drops_prefix_wrapper =
        expression_has_prefix_comment_or_doc_annotation_in_left_spine(context, inner_expression_id);
    let drops_ternary_wrapper = matches!(
        context.tree.get(normalized_inner_expression_id),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    );

    drops_tree_wrapper || drops_prefix_wrapper || drops_ternary_wrapper
}

/// Return whether a declaration heritage clause contains static type arguments.
fn declaration_has_generic_heritage(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let heritage = match context.tree.get(declaration_id) {
        Declaration::Struct { heritage, .. }
        | Declaration::Class { heritage, .. }
        | Declaration::Enum { heritage, .. }
        | Declaration::Interface { heritage, .. }
        | Declaration::Extension { heritage, .. } => heritage,
        _ => return false,
    };

    heritage.extends_types.as_ref().is_some_and(|types| {
        types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(context, type_id))
    }) || heritage.implements_types.as_ref().is_some_and(|types| {
        types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(context, type_id))
    })
}

/// Return whether a value expression wraps a class declaration with generic heritage.
fn value_has_generic_class_heritage(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            matches!(context.tree.get(*declaration_id), Declaration::Class { .. })
                && declaration_has_generic_heritage(context, *declaration_id)
        }
        Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. } => {
            value_has_generic_class_heritage(context, *left)
        }
        Expression::Parenthesized { expression } => {
            value_has_generic_class_heritage(context, *expression)
        }
        _ => false,
    }
}

/// Return whether a value expression wraps a class declaration.
fn value_is_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            matches!(context.tree.get(*declaration_id), Declaration::Class { .. })
        }
        Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. } => value_is_class_declaration(context, *left),
        Expression::Parenthesized { expression } => {
            value_is_class_declaration(context, *expression)
        }
        _ => false,
    }
}

/// Format a declarator (pattern, optional type, optional value).
pub(crate) fn format_declarator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    tree: &NodeTree,
    declarator_id: LocalNodeId<Declarator>,
) -> FormatResult<()> {
    let declarator = tree.get(declarator_id);
    let Declarator { pattern, ty, value } = declarator;

    let Some(value_id) = value else {
        let (header_nodes, _, _) = buffer_declarator_header(f, *pattern, *ty)?;
        if let Some(header) = f.intern_vec(header_nodes) {
            f.write_node(header);
        }
        return Ok(());
    };

    // header
    let (header_nodes, is_left_short, left_may_break) = buffer_declarator_header(f, *pattern, *ty)?;
    let header = f.intern_vec(header_nodes);
    let header = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        if let Some(header) = &header {
            f.write_node(header.clone());
        }

        Ok(())
    });

    // value shape
    let value_expr = tree.get(*value_id);
    let value_inner_id = transparent_inner_expression(f.context(), *value_id);
    let value_inner_expr = tree.get(value_inner_id);
    let value_breakable = is_expression_breakable(tree, value_expr);
    let value_is_binary = matches!(value_inner_expr, Expression::Binary { .. });
    let value_is_sequence = matches!(value_inner_expr, Expression::SequenceExpression { .. });
    let value_is_tree = matches!(value_inner_expr, Expression::TreeExpression { .. });
    let value_is_chain_root = is_chain_root(tree, value_inner_id);
    let value_is_chain = is_expression_chain(tree, value_inner_id) || value_is_chain_root;
    let value_is_call_like = matches!(
        value_inner_expr,
        Expression::Call { .. } | Expression::New { .. } | Expression::Instantiation { .. }
    );
    let value_is_declaration = matches!(value_inner_expr, Expression::Declaration(_));
    let value_handles_its_own_breaking = value_is_binary
        || value_is_sequence
        || value_is_tree
        || value_is_chain
        || value_is_call_like
        || value_is_declaration;

    // source layout
    let value_has_own_line_prefix_annotation =
        declarator_expression_has_own_line_prefix_annotation(f.context(), *value_id);
    let pattern_span = f.context().span(*pattern);
    let value_has_prefix_annotation = f.context().has_prefix_annotation(*value_id);
    let value_has_assignment_operator_prefix_annotation =
        declarator_value_has_assignment_operator_prefix_comment(f.context(), *value_id);
    let assignment_operator_comment_nodes =
        declarator_value_assignment_operator_comment_nodes(f.context(), *value_id);
    let value_has_assignment_operator_comment = !assignment_operator_comment_nodes.is_empty();
    let value_has_prefix_annotation_that_forces_break =
        value_has_prefix_annotation && !value_has_assignment_operator_prefix_annotation;
    let header_end = ty
        .map(|type_id| f.context().span(type_id).end)
        .unwrap_or(pattern_span.end);
    let value_span = f.context().span(*value_id);
    let between_span = if header_end < value_span.start {
        Some(Span::new(value_span.file, header_end, value_span.start))
    } else {
        None
    };
    let value_has_newline = f.context().has_newline(value_span);
    let pattern_has_newline = f.context().has_newline(pattern_span);
    let pattern_has_default_assignment = pattern_has_default_assignment(tree, *pattern);
    let pattern_has_comments_or_annotations = f.context().has_annotation(*pattern)
        || !f
            .context()
            .comments_in_range(pattern_span.start, pattern_span.end)
            .is_empty();
    let value_has_between_comment = between_span.is_some_and(|span| {
        !f.context()
            .comments_in_range(span.start, span.end)
            .is_empty()
    }) && !value_has_assignment_operator_prefix_annotation;
    let value_is_string_literal = matches!(
        value_inner_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    );
    let value_is_template_expression =
        matches!(value_inner_expr, Expression::TemplateExpression { .. });
    let value_is_await_expression = matches!(
        value_expr,
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    );
    let value_is_comptime_expression = matches!(value_expr, Expression::Comptime { .. });
    let value_is_keyword_expression = value_is_await_expression || value_is_comptime_expression;
    let value_is_class_declaration = value_is_class_declaration(f.context(), value_inner_id);
    let value_has_generic_class_heritage =
        value_has_generic_class_heritage(f.context(), value_inner_id);
    let value_chain_breaks_after_operator =
        value_is_chain && is_poorly_breakable_member_or_call_chain(f, value_inner_id);
    let value_is_lambda_like = declarator_value_is_lambda_like(f.context(), *value_id);
    let value_prefers_break_after_operator =
        assignment_rhs_prefers_break_after_operator(f, *value_id);
    let right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_assignment_operator_comments(f, *value_id, true)?;

        format_assignment_value(
            f,
            *value_id,
            value_has_assignment_operator_prefix_annotation,
            &assignment_operator_comment_nodes,
        )
    });

    // left-hand side breaks first for complex patterns
    let layout = if pattern_has_newline
        || pattern_has_nested_default_assignment(tree, *pattern)
        || pattern_has_comments_or_annotations
        || pattern_is_array_like(tree, *pattern)
        || pattern_is_complex_destructuring(tree, *pattern)
    {
        AssignmentLikeLayout::BreakLeftHandSide
    }
    // OXC: left sides that already break and feed a lambda rhs should stay on the lhs side
    else if (!is_left_short || left_may_break) && value_is_lambda_like {
        AssignmentLikeLayout::BreakLeftHandSide
    }
    // compact atomic rhs values stay attached to `=`
    else if value_is_template_expression
        || (value_is_keyword_expression
            && !value_has_between_comment
            && !value_has_prefix_annotation_that_forces_break)
        || value_is_string_literal
        || (value_is_class_declaration
            && !value_has_generic_class_heritage
            && !value_has_prefix_annotation_that_forces_break
            && !value_has_between_comment
            && !value_has_own_line_prefix_annotation)
    {
        AssignmentLikeLayout::NeverBreakAfterOperator
    }
    // operator-bound trivia and awkward rhs values break after `=`
    else if value_has_assignment_operator_comment
        || value_has_prefix_annotation_that_forces_break
        || value_has_between_comment
        || value_has_own_line_prefix_annotation
        || value_prefers_break_after_operator
        || value_has_generic_class_heritage
        || (!is_left_short && value_chain_breaks_after_operator)
        || (value_is_call_like && pattern_has_default_assignment)
    {
        AssignmentLikeLayout::BreakAfterOperator
    }
    // short and stable rhs values stay attached to `=`
    else if !left_may_break
        && (is_left_short
            || value_is_string_literal
            || value_is_template_expression
            || value_is_class_declaration)
    {
        AssignmentLikeLayout::NeverBreakAfterOperator
    }
    // self-breaking rhs values use the fluid assignment-like layout
    else if value_handles_its_own_breaking || value_breakable || value_has_newline {
        AssignmentLikeLayout::Fluid
    }
    // fall back to the default OXC-style layout
    else {
        AssignmentLikeLayout::Fluid
    };

    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if layout == AssignmentLikeLayout::BreakLeftHandSide {
            write!(f, [header])?;
        } else {
            write!(f, [group(&header)])?;
        }

        write!(f, [space(), token("=")])?;

        write_assignment_like_right(f, layout, &right)
    });

    write!(f, [group(&content)])
}

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declarator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        format_declarator(f, f.context().tree, node_id)?;

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

        Ok(())
    }
}
