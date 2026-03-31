use crate::format::call::{format_call_expression, format_instantiation_expression};
use crate::format::chain::{
    argument_value_id_if_present, has_line_comment_between_expressions, is_chain_root,
    is_expression_chain, transparent_inner_expression,
};
use crate::format::context::expression_has_own_line_prefix;
use crate::format::expression::{
    expression_has_prefix_comment_or_doc_annotation_in_left_spine,
    expression_has_static_type_arguments, is_expression_breakable, is_pattern_breakable,
};
use crate::format::operator::{
    flattened_binary_operand_count, write_expression_with_inline_prefix_annotations,
};
use crate::format::tree::tree_literal_should_expand;
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Argument, Declaration, Declarator, Expression, IfKind, LocalNodeId,
    NodeTree, Pattern, PatternField, ScalarLiteral, TokenType, TypeBinaryOperator,
};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{
    dedent, fits_expanded, format_with, group, hard_line_break, indent, soft_line_break_or_space,
    space, token,
};
use destack_fir::{format_args, write};
use destack_source::Span;

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

/// Return whether one declarator value has one prefix annotation on the `=` seam.
pub(crate) fn declarator_value_has_assignment_seam_prefix_comment(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    context
        .annotation_ids(value_id)
        .iter()
        .any(|annotation_id| {
            let annotation = context.annotation(*annotation_id);
            let annotation_position = match annotation {
                Annotation::Doc { position, .. } => position,
                _ => {
                    return false;
                }
            };

            if !matches!(
                annotation_position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ) {
                return false;
            }

            let previous_token_is_assign = context
                .annotation_previous_non_whitespace_token(*annotation_id)
                .is_some_and(|token| token.token.ty == TokenType::Assign);
            let next_token_is_assign = context
                .annotation_next_non_whitespace_token_type(*annotation_id)
                == Some(TokenType::Assign);

            !previous_token_is_assign && next_token_is_assign
        })
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

/// Decide whether a declarator can drop one parenthesized value wrapper.
pub(crate) fn declarator_drops_parenthesized_value_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(parenthesized_id) {
        return false;
    }

    if crate::format::expression::parenthesized_has_leading_inner_newline(
        context,
        parenthesized_id,
        inner_expression_id,
    ) {
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
            } if !tree_literal_should_expand(context, arguments, elements)
        );

    // keep left-spine prefix ownership intact when the declarator already owns the seam
    let drops_prefix_wrapper =
        expression_has_prefix_comment_or_doc_annotation_in_left_spine(context, inner_expression_id);

    drops_tree_wrapper || drops_prefix_wrapper
}

/// Return whether a value expression wraps a class declaration with generic heritage.
pub(crate) fn value_has_generic_class_heritage(
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
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            value_has_generic_class_heritage(context, *expression)
        }
        _ => false,
    }
}

/// Return whether a value is a single-line closure-cast style type binary.
pub(crate) fn value_is_inline_closure_cast_type_binary(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    if context.node_has_newline(value_id) {
        return false;
    }

    if !matches!(
        context.tree.get(value_id),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    ) {
        return false;
    }

    let mut current_id = value_id;
    loop {
        let current_span = context.span(current_id);
        let annotations = context.annotation_ids(current_id);
        let has_prefix = !annotations.is_empty()
            && annotations.iter().all(|annotation_id| {
                let annotation_is_prefix = matches!(
                    context.annotation(*annotation_id),
                    Annotation::Doc {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                );
                if !annotation_is_prefix {
                    return false;
                }

                let annotation_span = context.annotation_span(*annotation_id);
                annotation_span.start < current_span.start
            });
        if has_prefix {
            return true;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                Some(*expression)
            }
            Expression::TypeBinary { left, .. }
            | Expression::Binary { left, .. }
            | Expression::Call { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => Some(*left),
            _ => None,
        };
        let Some(next_id) = next_id else {
            return false;
        };
        current_id = next_id;
    }
}

const LONG_BINARY_OPERAND_COUNT_THRESHOLD: usize = 2;
const ASSIGNMENT_CHAIN_EQUALS_BREAK_WIDTH: u16 = 80;

/// Return whether one chain value has an instantiation in its left prefix.
fn value_chain_has_instantiation_prefix(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = transparent_inner_expression(context, expression_id);

    loop {
        current_id = transparent_inner_expression(context, current_id);

        match context.tree.get(current_id) {
            Expression::Instantiation { .. } => return true,
            Expression::Call { left, .. }
            | Expression::New { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => {
                current_id = *left;
            }
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                current_id = *expression;
            }
            _ => return false,
        }
    }
}

/// Return whether one static argument list contains block-like type expressions.
fn static_argument_list_has_block_expressions(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    static_arguments.iter().copied().any(|argument_id| {
        let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
            return false;
        };
        let value_id = transparent_inner_expression(context, value_id);

        matches!(
            context.tree.get(value_id),
            Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
        )
    })
}

/// Return whether one expression carries any static generic arguments.
fn expression_has_static_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_deref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Call {
            left,
            static_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            ..
        }
        | Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            expression_has_static_arguments(context, *left)
                || static_arguments
                    .as_deref()
                    .is_some_and(|arguments| !arguments.is_empty())
        }
        Expression::Index { left, .. } => expression_has_static_arguments(context, *left),
        Expression::Instantiation {
            left,
            static_arguments,
        } => expression_has_static_arguments(context, *left) || !static_arguments.is_empty(),
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            expression_has_static_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether one expression contains multiple chained call-like operations.
fn expression_has_nested_call_chain(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = transparent_inner_expression(context, expression_id);
    let mut call_like_count = 0usize;

    loop {
        current_id = transparent_inner_expression(context, current_id);

        match context.tree.get(current_id) {
            Expression::Call { left, .. }
            | Expression::New { left, .. }
            | Expression::Instantiation { left, .. } => {
                call_like_count += 1;
                if call_like_count >= 2 {
                    return true;
                }

                current_id = *left;
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => {
                current_id = *left;
            }
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                current_id = *expression;
            }
            _ => return false,
        }
    }
}

/// Return whether one expression carries block-like static generic arguments.
fn expression_has_block_static_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Path {
            static_arguments, ..
        } => static_arguments.as_deref().is_some_and(|arguments| {
            static_argument_list_has_block_expressions(context, arguments)
        }),
        Expression::Call {
            left,
            static_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            ..
        }
        | Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            expression_has_block_static_arguments(context, *left)
                || static_arguments.as_deref().is_some_and(|arguments| {
                    static_argument_list_has_block_expressions(context, arguments)
                })
        }
        Expression::Index { left, .. } => expression_has_block_static_arguments(context, *left),
        Expression::Instantiation {
            left,
            static_arguments,
        } => {
            expression_has_block_static_arguments(context, *left)
                || static_argument_list_has_block_expressions(context, static_arguments)
        }
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            expression_has_block_static_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether one expression is a single call-like value whose callee receiver is another member chain.
fn expression_is_single_call_with_member_chain_callee(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    let callee_id = match context.tree.get(expression_id) {
        Expression::Call { left, .. } | Expression::New { left, .. } => *left,
        _ => return false,
    };
    let callee_id = transparent_inner_expression(context, callee_id);

    let receiver_id = match context.tree.get(callee_id) {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. } => *left,
        _ => return false,
    };
    let receiver_id = transparent_inner_expression(context, receiver_id);

    matches!(
        context.tree.get(receiver_id),
        Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    )
}

/// Return whether one expression wraps a class declaration with heritage clauses.
fn expression_has_class_heritage(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => match context.tree.get(*declaration_id) {
            Declaration::Class { heritage, .. } => {
                heritage
                    .extends_types
                    .as_ref()
                    .is_some_and(|types| !types.is_empty())
                    || heritage
                        .implements_types
                        .as_ref()
                        .is_some_and(|types| !types.is_empty())
            }
            _ => false,
        },
        Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. } => expression_has_class_heritage(context, *left),
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            expression_has_class_heritage(context, *expression)
        }
        _ => false,
    }
}

/// Return whether one chain contains at least one private member hop.
fn expression_chain_has_private_member(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    match context.tree.get(expression_id) {
        Expression::PrivateMember { .. } => true,
        Expression::Member { left, .. }
        | Expression::Call { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::New { left, .. } => expression_chain_has_private_member(context, *left),
        Expression::Parenthesized { expression }
        | Expression::Statement(expression)
        | Expression::Await { expression }
        | Expression::AwaitMaybe { expression } => {
            expression_chain_has_private_member(context, *expression)
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

    // header: pattern + optional type
    let header = format_with(|f| {
        write!(f, [pattern])?;
        if let Some(ty_id) = ty {
            write!(f, [token(":"), space()])?;
            write_expression_with_inline_prefix_annotations(f, *ty_id)?;
        }
        Ok(())
    });

    let Some(value_id) = value else {
        write!(f, [header])?;
        return Ok(());
    };

    // value shape
    let value_expr = tree.get(*value_id);
    let value_inner_id = transparent_inner_expression(f.context(), *value_id);
    let value_inner_expr = tree.get(value_inner_id);
    let pattern_breakable = is_pattern_breakable(tree, *pattern);
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

    // source facts
    let value_is_inline_closure_cast_type_binary =
        value_is_inline_closure_cast_type_binary(f.context(), *value_id);
    let value_has_generic_class_heritage =
        value_has_generic_class_heritage(f.context(), value_inner_id);
    let value_has_own_line_prefix_annotation =
        expression_has_own_line_prefix(f.context(), *value_id);
    let pattern_span = f.context().span(*pattern);
    let value_has_prefix_annotation = f.context().has_prefix_annotation(*value_id);
    let value_has_assignment_seam_prefix_annotation =
        declarator_value_has_assignment_seam_prefix_comment(f.context(), *value_id);
    let value_has_prefix_annotation_that_forces_break = value_has_prefix_annotation
        && !value_is_inline_closure_cast_type_binary
        && !value_has_assignment_seam_prefix_annotation;
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
    let value_is_parenthesized = matches!(value_expr, Expression::Parenthesized { .. });
    let value_has_between_comment = between_span.is_some_and(|span| {
        !f.context()
            .comments_in_range(span.start, span.end)
            .is_empty()
    }) && !value_has_assignment_seam_prefix_annotation;
    let value_has_line_comment_between_operands = match value_inner_expr {
        Expression::Binary { left, right, .. } => {
            has_line_comment_between_expressions(f.context(), *left, *right)
        }
        _ => false,
    };
    let value_binary_operand_count = match value_inner_expr {
        Expression::Binary { operator, .. } => {
            flattened_binary_operand_count(f.context(), value_inner_id, *operator)
        }
        _ => 0,
    };
    let value_is_long_binary =
        value_is_binary && value_binary_operand_count > LONG_BINARY_OPERAND_COUNT_THRESHOLD;
    let value_is_string_literal = matches!(
        value_inner_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_))
    );
    let value_is_template_expression =
        matches!(value_inner_expr, Expression::TemplateExpression { .. });
    let value_has_instantiation_prefix =
        value_is_chain && value_chain_has_instantiation_prefix(f.context(), value_inner_id);
    let value_is_await_expression = matches!(
        value_expr,
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    );
    let value_is_comptime_expression = matches!(value_expr, Expression::Comptime { .. });
    let value_has_static_arguments = expression_has_static_arguments(f.context(), value_inner_id);
    let value_has_nested_call_chain = expression_has_nested_call_chain(f.context(), value_inner_id);
    let value_has_block_static_arguments =
        expression_has_block_static_arguments(f.context(), value_inner_id);
    let value_has_class_heritage = expression_has_class_heritage(f.context(), value_inner_id);

    // layout fragments
    let format_inline = format_with(|f| {
        write!(f, [header, space(), token("="), space(), *value_id])?;
        Ok(())
    });

    // expand inline if value is breakable
    let should_force_expand_value = is_expression_breakable(tree, tree.get(*value_id));
    let format_value_expanded = format_with(|f| {
        write!(
            f,
            [
                header,
                space(),
                token("="),
                space(),
                fits_expanded(&group(value_id).should_expand(should_force_expand_value)),
            ]
        )
    });

    // expand the header while keeping value inline
    let format_header_expanded = format_with(|f| {
        write!(
            f,
            [
                fits_expanded(&group(&header).should_expand(true)),
                space(),
                token("="),
                space(),
                *value_id,
            ]
        )
    });

    // expand and indent the value
    let format_indented = format_with(|f| {
        group(&format_args![
            header,
            space(),
            token("="),
            indent(&format_args![hard_line_break(), value_id])
        ])
        .format(f)
    });
    let format_break_after_operator_for_binary =
        format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let format_value_without_chain = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let can_format_call_without_chain = !f.context().has_annotation(*value_id)
                    && !f.context().has_annotation(value_inner_id)
                    && value_is_chain;
                if !can_format_call_without_chain {
                    write!(f, [*value_id])?;
                    return Ok(());
                }

                match f.context().tree.get(value_inner_id) {
                    Expression::Call { .. } => {
                        format_call_expression(f, value_inner_id)?;
                    }
                    Expression::Instantiation { .. } => {
                        format_instantiation_expression(f, value_inner_id)?;
                    }
                    _ => {
                        write!(f, [*value_id])?;
                    }
                }
                Ok(())
            });

            let break_after_operator = soft_line_break_or_space();

            if value_has_prefix_annotation_that_forces_break
                || value_has_between_comment
                || value_is_sequence
                || value_has_instantiation_prefix
            {
                write!(
                    f,
                    [group(&format_args![
                        header,
                        space(),
                        token("="),
                        indent(&format_args![
                            break_after_operator,
                            format_value_without_chain
                        ])
                    ])]
                )
            } else {
                let dedented_value = dedent(&format_value_without_chain);
                write!(
                    f,
                    [group(&format_args![
                        header,
                        space(),
                        token("="),
                        indent(&format_args![break_after_operator, dedented_value])
                    ])]
                )
            }
        });
    // keep closure cast type binaries inline in declarator rhs
    if value_is_inline_closure_cast_type_binary {
        write!(f, [format_inline])?;
        return Ok(());
    }

    // keep template rhs values inline in declarators
    if value_is_template_expression {
        write!(f, [format_inline])?;
        return Ok(());
    }

    // keep compact await and comptime rhs values inline when trivia free
    let should_keep_inline_keyword_rhs = (value_is_await_expression
        || value_is_comptime_expression)
        && !pattern_breakable
        && !value_has_between_comment
        && !value_has_prefix_annotation_that_forces_break;
    if should_keep_inline_keyword_rhs {
        write!(f, [format_inline])?;
        return Ok(());
    }

    // keep string rhs mostly inline
    if value_is_string_literal {
        if pattern_breakable && pattern_has_newline {
            write!(f, [format_header_expanded])?;
            return Ok(());
        }

        write!(f, [format_break_after_operator_for_binary])?;
        return Ok(());
    }

    // declaration rhs values with prefix trivia should break directly after `=`
    if value_is_declaration && value_has_prefix_annotation_that_forces_break {
        write!(f, [format_break_after_operator_for_binary])?;
        return Ok(());
    }

    // binary rhs operator break rules
    if value_is_long_binary {
        write!(f, [format_break_after_operator_for_binary])?;
        return Ok(());
    }

    // chain and binary values that already own their line breaking
    if value_handles_its_own_breaking {
        let has_forced_operator_break =
            value_has_prefix_annotation_that_forces_break || value_has_between_comment;
        if value_has_own_line_prefix_annotation {
            write!(f, [format_break_after_operator_for_binary])?;
            return Ok(());
        }

        if pattern_breakable {
            if has_forced_operator_break {
                write!(f, [format_break_after_operator_for_binary])?;
                return Ok(());
            }

            let should_break_after_operator_for_rhs = !pattern_has_newline
                && ((value_is_call_like && pattern_has_default_assignment)
                    || value_is_sequence
                    || value_has_line_comment_between_operands);
            if should_break_after_operator_for_rhs {
                write!(f, [format_break_after_operator_for_binary])?;
                return Ok(());
            }

            write!(f, [format_inline])?;
            return Ok(());
        }

        if has_forced_operator_break {
            write!(f, [format_break_after_operator_for_binary])?;
            return Ok(());
        }

        // complex static generic argument blocks already break inside the rhs
        if value_has_block_static_arguments {
            write!(f, [format_inline])?;
            return Ok(());
        }

        // class heritage wrappers own their internal multiline breaking
        if value_has_class_heritage && !value_has_generic_class_heritage {
            write!(f, [format_inline])?;
            return Ok(());
        }

        // poor chains, generic argument calls, and sequence like rhs shapes prefer operator seams
        let value_is_simple_static_argument_call =
            value_has_static_arguments && !value_has_nested_call_chain;
        let should_break_after_operator_for_rhs = value_has_line_comment_between_operands
            || value_is_sequence
            || value_has_generic_class_heritage
            || value_has_instantiation_prefix
            || value_is_simple_static_argument_call
            || (value_is_chain
                && !value_is_call_like
                && !value_has_newline
                && expression_chain_has_private_member(f.context(), value_inner_id));
        if should_break_after_operator_for_rhs {
            write!(f, [format_break_after_operator_for_binary])?;
            return Ok(());
        }

        let line_width = f.context().options.line_width;
        let should_break_after_operator_for_long_member_call = line_width
            <= ASSIGNMENT_CHAIN_EQUALS_BREAK_WIDTH
            && value_is_chain
            && value_is_call_like
            && !value_has_nested_call_chain
            && expression_is_single_call_with_member_chain_callee(f.context(), value_inner_id);
        if should_break_after_operator_for_long_member_call {
            write!(f, [format_break_after_operator_for_binary])?;
            return Ok(());
        }

        write!(f, [format_inline])?;
        return Ok(());
    }

    // layout matrix for non self breaking values: both sides breakable
    if pattern_breakable && value_breakable {
        if value_has_newline || pattern_has_newline {
            write!(f, [format_value_expanded])?;
        } else {
            write!(f, [format_inline])?;
        }
        return Ok(());
    }

    // layout matrix for non self breaking values: only pattern breakable
    if pattern_breakable {
        if pattern_has_newline {
            write!(f, [format_header_expanded])?;
        } else if pattern_has_nested_default_assignment(tree, *pattern)
            || pattern_has_comments_or_annotations
            || pattern_is_array_like(tree, *pattern)
        {
            write!(f, [format_break_after_operator_for_binary])?;
        } else {
            write!(f, [format_inline])?;
        }
        return Ok(());
    }

    // layout matrix for non self breaking values: only value breakable
    if value_breakable {
        let should_prefer_operator_break = value_has_between_comment
            || value_has_own_line_prefix_annotation
            || (value_is_declaration && value_has_newline);
        if should_prefer_operator_break {
            let should_keep_inline = !value_has_newline
                && !value_has_between_comment
                && !value_has_prefix_annotation_that_forces_break
                && !value_has_own_line_prefix_annotation;
            if should_keep_inline {
                write!(f, [format_inline])?;
            } else {
                write!(f, [format_indented])?;
            }
            return Ok(());
        }

        if value_is_parenthesized && value_has_newline {
            write!(f, [format_inline])?;
            return Ok(());
        }

        if value_has_newline || value_has_prefix_annotation_that_forces_break {
            write!(f, [format_value_expanded])?;
            return Ok(());
        }

        write!(f, [format_inline])?;
        return Ok(());
    }

    // layout matrix for non self breaking values: neither side breakable
    if value_has_generic_class_heritage && value_has_newline {
        write!(f, [format_indented])?;
    } else if value_is_parenthesized && value_has_newline {
        write!(f, [format_inline])?;
    } else {
        write!(f, [format_break_after_operator_for_binary])?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declarator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(
            f,
            [crate::format::annotation::prefix_annotations(
                f.context(),
                node_id
            )]
        )?;

        format_declarator(f, f.context().tree, node_id)?;

        write!(
            f,
            [crate::format::annotation::infix_or_postfix_annotations(
                f.context(),
                node_id
            )]
        )?;

        Ok(())
    }
}
