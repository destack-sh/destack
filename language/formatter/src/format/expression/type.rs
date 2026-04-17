use crate::format::annotation::{
    format_raw_comment, format_trailing_comment_slice, infix_or_postfix_annotations,
    prefix_annotations, write_annotation_sequence,
};
use crate::format::collection::literal::format_scalar_literal;
use crate::format::collection::{
    TrailingSeparator, format_block_nodes_with_ignore_ranges, separated_entries,
};
use crate::format::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    parameter_is_variadic, should_break_function_parameters, single_parameter_should_hug,
    write_function_header_prefix, write_generic_parameter_list, write_signature_hug_parameter_list,
    write_signature_parameter_list,
};
use crate::format::declaration::write_statement_terminator_after_anchor;
use crate::format::expression::format_type_template_literal;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Comment, Declaration, DecoratorPosition, Expression, FunctionSignature, GenericArgument, Key,
    Keyword, LocalNodeId, Mutability, NodeType, TokenSpan, TokenType, TupleElement, TypeExpression,
    TypeMember, TypeModifier, TypePredicateSubject, VarianceBound,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{space, token, *};
use destack_fir::{format_args, write};
use destack_source::NodeSpanType;

/// Return the innermost type that can own one postfix type operator.
fn normalize_postfix_type_operand(
    context: &DestackFormatContext<'_>,
    mut expression_id: LocalNodeId<TypeExpression>,
) -> LocalNodeId<TypeExpression> {
    loop {
        // transparent grouped wrappers
        while let TypeExpression::Parenthesized { expression } = context.tree.get(expression_id) {
            if context.has_prefix_annotation(expression_id)
                || context.has_infix_annotation(expression_id)
            {
                return expression_id;
            }

            expression_id = *expression;
        }

        // single member unions and intersections are transparent here
        match context.tree.get(expression_id) {
            TypeExpression::Union { elements } | TypeExpression::Intersection { elements }
                if elements.len() == 1 =>
            {
                expression_id = elements[0];
            }
            _ => return expression_id,
        }
    }
}

/// Return whether one normalized type needs parentheses before postfix operators.
fn type_needs_postfix_parentheses(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(expression_id) {
        TypeExpression::Union { elements } => elements.len() > 1,
        TypeExpression::Intersection { elements } => elements.len() > 1,
        TypeExpression::Conditional { .. }
        | TypeExpression::Mapped { .. }
        | TypeExpression::Readonly { .. }
        | TypeExpression::KeyOf { .. }
        | TypeExpression::TypeOfValue { .. }
        | TypeExpression::Must { .. }
        | TypeExpression::AsComptime { .. }
        | TypeExpression::Not { .. }
        | TypeExpression::ValueOf { .. }
        | TypeExpression::ReferenceOf { .. }
        | TypeExpression::PointerOf { .. }
        | TypeExpression::Infer { .. }
        | TypeExpression::Predicate { .. } => true,
        TypeExpression::Declaration { declaration } => {
            matches!(context.tree.get(*declaration), Declaration::Function(_))
        }
        _ => false,
    }
}

/// One summary of union-leading comment ownership.
#[derive(Debug, Clone, Copy, Default)]
struct LeadingCommentsInfo {
    has_own_line_comment: bool,
    has_end_of_line_comment: bool,
    has_trailing_own_line_non_jsdoc_block_comment: bool,
    has_trailing_own_line_jsdoc_comment: bool,
}

impl LeadingCommentsInfo {
    /// Build one leading-comment summary from raw comments.
    fn from_comment_nodes(context: &DestackFormatContext<'_>, comments: &[Comment]) -> Self {
        let mut info = Self::default();

        for comment in comments.iter().copied() {
            let is_doc_comment = context.comment_is_doc(comment);
            let is_trailing_comment = !comment.preceded_by_newline();

            info.has_own_line_comment |= comment.preceded_by_newline();
            info.has_end_of_line_comment |= comment.followed_by_newline();
            info.has_trailing_own_line_non_jsdoc_block_comment |= comment.is_block()
                && is_trailing_comment
                && comment.followed_by_newline()
                && !is_doc_comment;
            info.has_trailing_own_line_jsdoc_comment |=
                is_trailing_comment && comment.followed_by_newline() && is_doc_comment;
        }

        info
    }
}

/// Return leading-comment info normalized for one union head owner.
fn union_leading_comment_info(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    comments: &[Comment],
) -> LeadingCommentsInfo {
    let mut info = LeadingCommentsInfo::from_comment_nodes(context, comments);

    if !comments.is_empty()
        && matches!(
            context.tree.get(node_id),
            TypeExpression::Parenthesized { .. }
        )
    {
        info.has_trailing_own_line_non_jsdoc_block_comment = false;
        info.has_trailing_own_line_jsdoc_comment = false;
    }

    info
}

/// Write leading union comments while preserving source separators.
fn write_union_leading_comment_nodes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    for (index, comment) in comments.iter().copied().enumerate() {
        format_raw_comment(f, comment)?;

        if let Some(next_comment) = comments.get(index + 1).copied() {
            let next_comment_starts_on_own_line =
                f.context().span_starts_on_own_line(next_comment.span);

            if comment.is_line() || next_comment_starts_on_own_line {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [space()])?;
            }
        }
    }

    Ok(())
}

/// Return whether separator comments force a following break.
fn union_separator_comments_require_break_after(comments: &[Comment]) -> bool {
    comments.last().is_some_and(|comment| comment.is_line())
}

/// Split boundary-owned union comments into head comments and first-separator comments.
fn split_union_boundary_comment_groups(
    context: &DestackFormatContext<'_>,
    comments: Vec<Comment>,
) -> (Vec<Comment>, Vec<Comment>) {
    let mut head_comments = comments;
    let mut first_separator_comments = Vec::new();

    while let Some(last_comment) = head_comments.last().copied() {
        let Some(next_token) = context.next_non_whitespace_token_after_span(last_comment.span)
        else {
            break;
        };

        if next_token.token.ty != TokenType::ElementwiseOr
            || !context.span_starts_on_own_line(last_comment.span)
            || union_separator_comments_require_break_after(&[last_comment])
        {
            break;
        }

        first_separator_comments.push(
            head_comments
                .pop()
                .expect("last comment exists when splitting union boundary comments"),
        );
    }

    first_separator_comments.reverse();
    (head_comments, first_separator_comments)
}

/// Return whether one first-separator comment slice can stay inline with the union head.
fn union_separator_comments_can_stay_inline(
    context: &DestackFormatContext<'_>,
    comments: &[Comment],
) -> bool {
    !comments.is_empty()
        && comments.iter().copied().all(|comment| {
            let comment_span = comment.span;

            comment.is_block()
                && !context.has_newline(comment_span)
                && !context.span_starts_on_own_line(comment_span)
                && !context.span_has_newline_before_next_non_whitespace_token(comment_span)
        })
}

/// Return raw comments between one union separator and its operand body.
fn union_comments_after_separator(
    context: &DestackFormatContext<'_>,
    element_id: LocalNodeId<TypeExpression>,
) -> Vec<Comment> {
    let element_span = context.span(element_id);
    let Some(separator_token) = context.previous_non_trivia_token_before_span(element_span) else {
        return Vec::new();
    };

    if separator_token.token.ty != TokenType::ElementwiseOr {
        return Vec::new();
    }

    let comment_start = separator_token.span.end;
    let comment_end = context.type_expression_token_start(element_id);

    if comment_start >= comment_end {
        return Vec::new();
    }

    context
        .tree
        .comments()
        .iter()
        .copied()
        .filter(|comment| {
            comment.span.file == element_span.file
                && comment.span.start >= comment_start
                && comment.span.start < comment_end
        })
        .collect()
}

/// Return raw comments between one union operand body and the following separator.
fn union_comments_before_separator(
    context: &DestackFormatContext<'_>,
    previous_element_id: LocalNodeId<TypeExpression>,
    element_id: LocalNodeId<TypeExpression>,
) -> Vec<Comment> {
    let previous_element_span = context.span(previous_element_id);
    let current_element_span = context.span(element_id);
    let Some(separator_token) = context.previous_non_trivia_token_before_span(current_element_span)
    else {
        return Vec::new();
    };

    if separator_token.token.ty != TokenType::ElementwiseOr {
        return Vec::new();
    }

    let comment_start = previous_element_span.end;
    let comment_end = separator_token.span.start;

    if comment_start >= comment_end {
        return Vec::new();
    }

    context
        .tree
        .comments()
        .iter()
        .copied()
        .filter(|comment| {
            comment.span.file == previous_element_span.file
                && comment.span.start >= comment_start
                && comment.span.start < comment_end
        })
        .collect()
}

/// Return the direct leading `|` token that owns the first union operand.
fn union_root_direct_leading_separator_token(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    element_id: LocalNodeId<TypeExpression>,
) -> Option<TokenSpan> {
    let leading_span = context.tree.get_side_span(node_id, NodeSpanType::Leading)?;
    let element_start = context.type_expression_token_start(element_id);

    context
        .non_trivia_tokens_in_span(leading_span)
        .into_iter()
        .rev()
        .find(|token| {
            token.token.ty == TokenType::ElementwiseOr && token.span.start < element_start
        })
}

/// Return raw comments between one wrapper-owned leading `|` and the first operand body.
fn union_root_comments_after_leading_separator(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    element_id: LocalNodeId<TypeExpression>,
) -> Vec<Comment> {
    let Some(leading_span) = context.tree.get_side_span(node_id, NodeSpanType::Leading) else {
        return Vec::new();
    };
    let Some(separator_token) =
        union_root_direct_leading_separator_token(context, node_id, element_id)
    else {
        return Vec::new();
    };

    let comment_start = separator_token.span.end;
    let comment_end = leading_span
        .end
        .min(context.type_expression_token_start(element_id));

    if comment_start >= comment_end {
        return Vec::new();
    }

    context
        .tree
        .comments()
        .iter()
        .copied()
        .filter(|comment| {
            comment.span.file == leading_span.file
                && comment.span.start >= comment_start
                && comment.span.start < comment_end
        })
        .collect()
}

/// Return head comments and first-separator comments owned by one union root shell.
fn union_root_leading_comment_groups(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    first_element_id: LocalNodeId<TypeExpression>,
) -> (Vec<Comment>, Vec<Comment>) {
    if let Some(leading_span) = context.tree.get_side_span(node_id, NodeSpanType::Leading) {
        if let Some(separator_token) =
            union_root_direct_leading_separator_token(context, node_id, first_element_id)
        {
            let head_comments = context
                .raw_boundary_comments_in_range(leading_span.start, separator_token.span.start);
            let separator_comments =
                union_root_comments_after_leading_separator(context, node_id, first_element_id);

            return (head_comments, separator_comments);
        }

        let leading_span_comments =
            context.raw_boundary_comments_in_range(leading_span.start, leading_span.end);
        if !leading_span_comments.is_empty() {
            return (leading_span_comments, Vec::new());
        }
    }

    (context.raw_prefix_comments_for(node_id), Vec::new())
}

/// Return whether one type is simple enough to keep inline inside unions.
fn union_element_is_simple(expression: &TypeExpression) -> bool {
    matches!(
        expression,
        TypeExpression::Parenthesized { .. }
            | TypeExpression::ScalarLiteral { .. }
            | TypeExpression::Literal { .. }
            | TypeExpression::Intrinsic
            | TypeExpression::Tuple { .. }
            | TypeExpression::Array { .. }
            | TypeExpression::Declaration { .. }
            | TypeExpression::Reference { .. }
            | TypeExpression::Import { .. }
            | TypeExpression::Readonly { .. }
            | TypeExpression::KeyOf { .. }
            | TypeExpression::TypeOfValue { .. }
            | TypeExpression::Must { .. }
            | TypeExpression::AsComptime { .. }
            | TypeExpression::Not { .. }
            | TypeExpression::ValueOf { .. }
            | TypeExpression::ReferenceOf { .. }
            | TypeExpression::PointerOf { .. }
            | TypeExpression::Index { .. }
            | TypeExpression::TemplateLiteral { .. }
            | TypeExpression::Infer { .. }
            | TypeExpression::Predicate { .. }
            | TypeExpression::This
            | TypeExpression::Member { .. }
            | TypeExpression::Object { .. }
    )
}

/// Flatten nested union wrappers into one operand list.
fn flatten_union_elements(
    context: &DestackFormatter<'_, '_>,
    expression_id: LocalNodeId<TypeExpression>,
    elements: &mut Vec<LocalNodeId<TypeExpression>>,
) {
    let mut expression_id = expression_id;

    loop {
        let TypeExpression::Parenthesized { expression } =
            context.context().tree.get(expression_id)
        else {
            break;
        };

        if context.context().has_prefix_annotation(expression_id)
            || context.context().has_infix_annotation(expression_id)
        {
            break;
        }

        expression_id = *expression;
    }

    if let TypeExpression::Union {
        elements: inner_elements,
    } = context.context().tree.get(expression_id)
    {
        for inner_element_id in inner_elements.iter().copied() {
            flatten_union_elements(context, inner_element_id, elements);
        }

        return;
    }

    elements.push(expression_id);
}

/// Return whether one union should stay inline when it fits.
fn union_should_hug(
    f: &DestackFormatter<'_, '_>,
    node_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
    leading_comments: &[Comment],
    first_separator_comments: &[Comment],
) -> bool {
    if elements.is_empty() {
        return true;
    }

    if f.context().has_prefix_annotation(node_id)
        || !leading_comments.is_empty()
        || !first_separator_comments.is_empty()
    {
        return false;
    }

    for element_id in elements.iter().copied().skip(1) {
        if !union_comments_after_separator(f.context(), element_id).is_empty() {
            return false;
        }
    }

    for (index, element_id) in elements.iter().copied().enumerate() {
        let element = f.context().tree.get(element_id);
        let element_span = f.context().span(element_id);
        let is_last_element = index + 1 == elements.len();

        if !union_element_is_simple(element) {
            return false;
        }

        if !is_last_element
            && !f
                .context()
                .end_of_line_raw_comments_after(element_span.end)
                .is_empty()
        {
            return false;
        }
    }

    true
}

/// Return whether one multiline union should own one extra indent.
fn union_should_indent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    leading_comment_info: LeadingCommentsInfo,
) -> bool {
    let Some((parent_id, parent_type)) = union_indent_owner(context, node_id) else {
        return false;
    };

    match parent_type {
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(parent_id);

            match context.tree.get(declaration_id) {
                // type aliases already own the assignment shell, except for the jsdoc edge case
                Declaration::Type(_) => !leading_comment_info.has_trailing_own_line_jsdoc_comment,

                // other declarations follow the default union shell
                _ => true,
            }
        }

        // tuple slots already indent their element body
        NodeType::TupleElement => false,

        // direct type-argument owners already indent their value wrapper
        NodeType::GenericArgument => false,

        // expression owners need one more shape-based split
        NodeType::Expression => {
            let expression_id = LocalNodeId::<Expression>::new(parent_id);

            union_expression_should_indent(context, expression_id)
        }

        // other owners use the default union shell
        _ => true,
    }
}

/// Return the outer owner that decides one union indent shell.
fn union_indent_owner(
    context: &DestackFormatContext<'_>,
    mut node_id: LocalNodeId<TypeExpression>,
) -> Option<(u32, NodeType)> {
    loop {
        let (parent_id, parent_type) = context.parent(node_id)?;

        // grouped wrappers are transparent here
        if parent_type == NodeType::TypeExpression {
            let parent_type_id = LocalNodeId::<TypeExpression>::new(parent_id);

            if matches!(
                context.tree.get(parent_type_id),
                TypeExpression::Parenthesized { .. }
            ) {
                node_id = parent_type_id;
                continue;
            }
        }

        return Some((parent_id, parent_type));
    }
}

/// Return whether one expression owner should add one union indent shell.
fn union_expression_should_indent(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        // generic arguments already indent their explicit type wrapper
        Expression::Type { .. } => context
            .parent(expression_id)
            .is_none_or(|(_, parent_type)| parent_type != NodeType::GenericArgument),

        // cast-like expressions already indent their type side
        Expression::As { .. } | Expression::Satisfies { .. } => false,

        // other expression owners use the default union shell
        _ => true,
    }
}

/// Write one inline union body without multiline ownership logic.
fn write_inline_union_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<TypeExpression>],
) -> FormatResult<()> {
    for (index, element_id) in elements.iter().copied().enumerate() {
        // separator
        if index > 0 {
            write!(f, [space(), token("|"), space()])?;
        }

        // operand
        write!(f, [element_id])?;
    }

    // trailing comments
    if let Some(last_element_id) = elements.last().copied() {
        let trailing_comments = {
            let comments = f.context().comments();
            comments
                .end_of_line_comments_after(f.context().span(last_element_id).end)
                .to_vec()
        };

        if !trailing_comments.is_empty() {
            write!(f, [format_trailing_comment_slice(&trailing_comments)])?;
        }
    }

    Ok(())
}

/// Write one union type with break-aware leading separators.
fn write_union_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
    only_type: bool,
) -> FormatResult<()> {
    let mut flattened_elements = Vec::new();

    // flatten wrappers
    for element_id in elements.iter().copied() {
        flatten_union_elements(f, element_id, &mut flattened_elements);
    }

    // empty union
    if flattened_elements.is_empty() {
        return Ok(());
    }

    // boundary comments
    let first_element_id = flattened_elements[0];
    let (root_leading_comments, mut first_separator_comments) =
        union_root_leading_comment_groups(f.context(), node_id, first_element_id);
    let (mut leading_comments, root_first_separator_comments) =
        split_union_boundary_comment_groups(f.context(), root_leading_comments);
    let boundary_comments = f.context().raw_type_position_comments_for(node_id);
    let (boundary_head_comments, boundary_first_separator_comments) =
        split_union_boundary_comment_groups(f.context(), boundary_comments);

    first_separator_comments.extend(root_first_separator_comments);

    // inline separator comments stay with the head
    if union_separator_comments_can_stay_inline(f.context(), &first_separator_comments) {
        leading_comments.append(&mut first_separator_comments);
    }

    // boundary-owned comments extend the root-owned shell
    leading_comments.extend(boundary_head_comments);
    first_separator_comments.extend(boundary_first_separator_comments);
    leading_comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
    leading_comments.dedup_by_key(|comment| (comment.span.start, comment.span.end));
    first_separator_comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
    first_separator_comments.dedup_by_key(|comment| (comment.span.start, comment.span.end));

    first_separator_comments.retain(|comment| {
        !leading_comments.iter().copied().any(|leading_comment| {
            leading_comment.span.start == comment.span.start
                && leading_comment.span.end == comment.span.end
        })
    });

    // inline unions
    let leading_comment_info = union_leading_comment_info(f.context(), node_id, &leading_comments);
    let should_hug = union_should_hug(
        f,
        node_id,
        &flattened_elements,
        &leading_comments,
        &first_separator_comments,
    );

    if should_hug {
        return write_inline_union_type(f, &flattened_elements);
    }

    // multiline indent
    let should_indent = union_should_indent(f.context(), node_id, leading_comment_info);

    // grouped content
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // head shell
        if !should_hug {
            let has_own_line_comment = leading_comment_info.has_own_line_comment;
            let is_parenthesized_wrapper = matches!(
                f.context().tree.get(node_id),
                TypeExpression::Parenthesized { .. }
            );
            let first_comment_starts_on_own_line = leading_comments
                .first()
                .is_some_and(|comment| f.context().span_starts_on_own_line(comment.span));
            let should_break_before_leading_comments = (has_own_line_comment && !only_type)
                || (is_parenthesized_wrapper && !leading_comments.is_empty())
                || (first_comment_starts_on_own_line && only_type);
            let should_force_hard_break_before_leading_comments =
                has_own_line_comment || (first_comment_starts_on_own_line && only_type);

            if should_break_before_leading_comments {
                if should_force_hard_break_before_leading_comments {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [soft_line_break()])?;
                }
            }

            if !leading_comments.is_empty() {
                write_union_leading_comment_nodes(f, &leading_comments)?;
                f.context()
                    .comments_mut()
                    .skip_comments_before(f.context().span(first_element_id).start);

                if !first_separator_comments.is_empty()
                    && leading_comments.last().is_some_and(|comment| {
                        comment.is_line()
                            || f.context()
                                .span_has_newline_before_next_non_whitespace_token(comment.span)
                    })
                {
                    write!(f, [hard_line_break()])?;
                }
            }

            if !leading_comment_info.has_end_of_line_comment && has_own_line_comment && only_type {
                write!(f, [hard_line_break()])?;
            }

            if first_separator_comments.is_empty() {
                write!(
                    f,
                    [if_group_breaks(&format_args![
                        soft_line_break_or_space(),
                        token("|"),
                        space()
                    ])]
                )?;
            }
        }

        for (index, element_id) in flattened_elements.iter().copied().enumerate() {
            // following operands
            if index > 0 {
                let previous_element_id = flattened_elements[index - 1];
                let current_element_token_start =
                    f.context().type_expression_token_start(element_id);
                let previous_trailing_comments =
                    union_comments_before_separator(f.context(), previous_element_id, element_id);
                let separator_comments = union_comments_after_separator(f.context(), element_id);

                if !previous_trailing_comments.is_empty() {
                    write!(
                        f,
                        [format_trailing_comment_slice(&previous_trailing_comments)]
                    )?;
                    write!(f, [hard_line_break()])?;
                    f.context()
                        .comments_mut()
                        .skip_comments_before(current_element_token_start);
                } else if !separator_comments.is_empty() {
                    write!(f, [format_trailing_comment_slice(&separator_comments)])?;
                    write!(f, [hard_line_break()])?;
                    f.context()
                        .comments_mut()
                        .skip_comments_before(current_element_token_start);
                } else {
                    write!(f, [soft_line_break_or_space()])?;
                }

                write!(f, [token("|"), space()])?;
            // first separator comments
            } else if !first_separator_comments.is_empty() {
                write_union_leading_comment_nodes(f, &first_separator_comments)?;
                f.context()
                    .comments_mut()
                    .skip_comments_before(f.context().span(element_id).start);
                write!(f, [space(), token("|")])?;

                if union_separator_comments_require_break_after(&first_separator_comments) {
                    write!(f, [hard_line_break(), space(), space()])?;
                } else {
                    write!(f, [space()])?;
                }
            }

            // operand
            write!(f, [element_id])?;
        }

        // trailing comments
        if let Some(last_element_id) = flattened_elements.last().copied() {
            let trailing_comments = {
                let comments = f.context().comments();
                comments
                    .end_of_line_comments_after(f.context().span(last_element_id).end)
                    .to_vec()
            };

            if !trailing_comments.is_empty() {
                write!(f, [format_trailing_comment_slice(&trailing_comments)])?;
            }
        }

        Ok(())
    });

    let content = group(&content).should_expand(!should_hug);

    if should_indent {
        write!(f, [group(&indent(&content))])
    } else {
        write!(f, [content])
    }
}

/// Return the union wrapped by redundant parenthesized shells when one exists.
fn parenthesized_union_elements<'ast, 'buf>(
    context: &DestackFormatter<'ast, 'buf>,
    expression_id: LocalNodeId<TypeExpression>,
) -> Option<&'ast [LocalNodeId<TypeExpression>]> {
    let mut expression_id = expression_id;
    let mut wrapper_count = 1;

    loop {
        let TypeExpression::Parenthesized { expression } =
            context.context().tree.get(expression_id)
        else {
            break;
        };

        if context.context().has_prefix_annotation(expression_id)
            || context.context().has_infix_annotation(expression_id)
        {
            return None;
        }

        wrapper_count += 1;
        expression_id = *expression;
    }

    if wrapper_count <= 1 {
        return None;
    }

    match context.context().tree.get(expression_id) {
        TypeExpression::Union { elements } => Some(elements.as_slice()),
        _ => None,
    }
}

/// Return whether one type body owns its raw prefix comments.
fn type_expression_body_owns_prefix_comments(
    f: &DestackFormatter<'_, '_>,
    expression: &TypeExpression,
) -> bool {
    match expression {
        TypeExpression::Union { .. } => true,
        TypeExpression::Parenthesized { expression } => {
            parenthesized_union_elements(f, *expression).is_some()
        }
        _ => false,
    }
}

/// Write prefix annotations for one type expression.
fn write_type_expression_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
) -> FormatResult<()> {
    // body-owned comments
    if type_expression_body_owns_prefix_comments(f, expression) {
        let mut annotation_ids = Vec::new();

        for annotation_id in f.context().annotation_ids(node_id).iter().copied() {
            let position = f.context().annotation(annotation_id).position;

            if matches!(
                position,
                DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
            ) {
                annotation_ids.push(annotation_id);
            }
        }

        return write_annotation_sequence(f, &annotation_ids);
    }

    // regular prefix ownership
    write!(f, [prefix_annotations(f.context(), node_id)])
}

/// Write one type with parentheses when one postfix operator needs them.
fn write_postfix_type_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let expression_id = normalize_postfix_type_operand(f.context(), expression_id);

    // postfix grouping
    if type_needs_postfix_parentheses(f.context(), expression_id) {
        write!(f, [token("("), expression_id, token(")")])
    } else {
        write!(f, [expression_id])
    }
}

/// Write one generic-argument list in type position.
fn write_generic_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<()> {
    // empty list
    if generic_arguments.is_empty() {
        return write!(f, [token("<>")]);
    }

    // grouped list
    let body = separated_entries(",", generic_arguments, TrailingSeparator::Omit, None);
    write!(
        f,
        [group(&format_args![
            token("<"),
            soft_block_indent(&body),
            token(">")
        ])]
    )
}

/// Write one list of callable parameters in type position.
fn write_type_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    signature: &FunctionSignature,
) -> FormatResult<()> {
    let mut parameters = Vec::with_capacity(signature.parameters.len() + 1);

    // this parameter
    if let Some(this_parameter) = signature.this_parameter {
        parameters.push(this_parameter);
    }

    // regular parameters
    parameters.extend(signature.parameters.iter().copied());

    // empty list
    if parameters.is_empty() {
        return write!(f, [token("("), token(")")]);
    }

    // hugging
    if parameters.len() == 1 && single_parameter_should_hug(f.context(), parameters[0]) {
        return write_signature_hug_parameter_list(f, &parameters);
    }

    // grouped list
    let should_break = should_break_function_parameters(f.context(), &parameters);
    let disallow_trailing_parameter_separator = parameters
        .last()
        .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id));

    write_signature_parameter_list(
        f,
        &parameters,
        should_break,
        disallow_trailing_parameter_separator,
    )
}

/// Write one type-space function signature.
fn write_type_signature<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    signature: &FunctionSignature,
    key: Option<Key>,
    is_optional: bool,
) -> FormatResult<()> {
    let has_name_or_key = key.is_some();

    // header
    write_function_header_prefix(f, signature, false, has_name_or_key)?;

    // key
    if let Some(key) = key {
        match key {
            Key::Expression(expression) => {
                write!(f, [token("["), expression, token("]")])?;
            }
            _ => {
                write!(f, [key])?;
            }
        }

        if is_optional {
            write!(f, [token("?")])?;
        }
    }

    // generic parameters
    if !signature.generic_parameters.is_empty() {
        write_generic_parameter_list(
            f,
            &signature.generic_parameters,
            default_generic_parameter_trailing_separator(f),
        )?;
    }

    // parameters
    write_type_parameters(f, signature)?;

    // return type
    if let Some(return_type) = signature.return_type {
        write!(f, [space(), token(":"), space(), return_type])?;
    }

    // where clauses
    if !signature.where_clauses.is_empty() {
        format_where_clause_with_break(f, &signature.where_clauses)?;
    }

    Ok(())
}

/// Write one mapped-type modifier prefix.
fn write_mapped_modifier_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifier: TypeModifier,
    keyword: &'static str,
) -> FormatResult<()> {
    // modifier
    match modifier {
        TypeModifier::Present => write!(f, [token(keyword), space()]),
        TypeModifier::Add => write!(f, [token("+"), token(keyword), space()]),
        TypeModifier::Remove => write!(f, [token("-"), token(keyword), space()]),
        TypeModifier::None => Ok(()),
    }
}

/// Write one mapped-type modifier suffix.
fn write_mapped_modifier_suffix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifier: TypeModifier,
) -> FormatResult<()> {
    // modifier
    match modifier {
        TypeModifier::Present => write!(f, [token("?")]),
        TypeModifier::Add => write!(f, [token("+?")]),
        TypeModifier::Remove => write!(f, [token("-?")]),
        TypeModifier::None => Ok(()),
    }
}

/// Format a block of type members with ignore-range handling.
pub(crate) fn format_block_of_type_members<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<TypeMember>],
) -> FormatResult<()> {
    format_block_nodes_with_ignore_ranges(f, members, |f, member_id| {
        write!(f, [member_id])?;

        write_statement_terminator_after_anchor(f, f.context().span(member_id).end)
    })
}

/// Write one type body without node-owned annotations.
fn write_type_expression_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
) -> FormatResult<()> {
    match expression {
        TypeExpression::Parenthesized { expression } => {
            if let Some(elements) = parenthesized_union_elements(f, node_id)
                && elements.len() == 1
            {
                write_union_type(f, node_id, elements, true)?;
                return Ok(());
            }

            write!(f, [token("("), expression, token(")")])?;
        }
        TypeExpression::ScalarLiteral { value } => {
            format_scalar_literal(value, f.context().span(node_id), f)?;
        }
        TypeExpression::Literal { value } => {
            write!(f, [value])?;
        }
        TypeExpression::Intrinsic => {
            write!(f, [token("intrinsic")])?;
        }
        TypeExpression::Tuple { elements } => {
            let body = separated_entries(",", elements, TrailingSeparator::Omit, None);
            write!(
                f,
                [group(&format_args![
                    token("["),
                    soft_block_indent(&body),
                    token("]")
                ])]
            )?;
        }
        TypeExpression::Array { element } => {
            write_postfix_type_operand(f, *element)?;
            write!(f, [token("[]")])?;
        }
        TypeExpression::Object { members } => {
            // empty body
            if members.is_empty() {
                if f.context().options.bracket_spacing {
                    write!(f, [token("{"), space(), token("}")])?;
                } else {
                    write!(f, [token("{}")])?;
                }
                return Ok(());
            }

            // grouped body
            let body = separated_entries(";", members, TrailingSeparator::Omit, None);
            write!(
                f,
                [group(&format_args![
                    token("{"),
                    soft_block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        write!(f, [body])?;

                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        Ok(())
                    })),
                    token("}")
                ])]
            )?;
        }
        TypeExpression::Declaration { declaration } => {
            write!(f, [declaration])?;
        }
        TypeExpression::Reference {
            path,
            generic_arguments,
        } => {
            write!(f, [path])?;

            if !generic_arguments.is_empty() {
                write_generic_argument_list(f, generic_arguments)?;
            }
        }
        TypeExpression::Member {
            left,
            name,
            generic_arguments,
        } => {
            write_postfix_type_operand(f, *left)?;
            write!(f, [token("."), *name])?;

            if !generic_arguments.is_empty() {
                write_generic_argument_list(f, generic_arguments)?;
            }
        }
        TypeExpression::Const => {
            write!(f, [Keyword::Const])?;
        }
        TypeExpression::This => {
            write!(f, [Keyword::This])?;
        }
        TypeExpression::Import {
            target,
            arguments,
            qualifier,
            generic_arguments,
        } => {
            let format_arguments = format_with(|f| {
                write!(f, [target])?;

                if !arguments.is_empty() {
                    write!(f, [token(","), space()])?;
                    write!(
                        f,
                        [separated_entries(
                            ",",
                            arguments,
                            TrailingSeparator::Omit,
                            None,
                        )]
                    )?;
                }

                Ok(())
            });

            write!(
                f,
                [
                    Keyword::Import,
                    group(&format_args![
                        token("("),
                        soft_block_indent(&format_arguments),
                        token(")")
                    ])
                ]
            )?;

            if let Some(qualifier) = qualifier {
                write!(f, [token("."), qualifier])?;
            }

            if !generic_arguments.is_empty() {
                write_generic_argument_list(f, generic_arguments)?;
            }
        }
        TypeExpression::Readonly { target_type } => {
            write!(f, [Keyword::Readonly, space(), target_type])?;
        }
        TypeExpression::KeyOf { target_type } => {
            write!(f, [Keyword::Keyof, space(), target_type])?;
        }
        TypeExpression::TypeOfValue { value } => {
            write!(f, [Keyword::Typeof, space(), value])?;
        }
        TypeExpression::Must { target_type } => {
            write!(f, [target_type, token("!")])?;
        }
        TypeExpression::AsComptime { target_type } => {
            write!(
                f,
                [
                    target_type,
                    space(),
                    token("as"),
                    space(),
                    Keyword::Comptime
                ]
            )?;
        }
        TypeExpression::Not { target_type } => {
            write!(f, [token("!"), target_type])?;
        }
        TypeExpression::ValueOf {
            mutability,
            variance,
            target_type,
        } => {
            write!(f, [token("^")])?;

            if *mutability == Some(Mutability::Immutable) {
                write!(f, [Keyword::Readonly, space()])?;
            }

            if let Some(variance) = variance {
                match variance {
                    VarianceBound::Implements => write!(f, [Keyword::Implements, space()])?,
                    VarianceBound::Extends => write!(f, [Keyword::Extends, space()])?,
                    VarianceBound::Super => write!(f, [Keyword::Super, space()])?,
                }
            }

            write!(f, [target_type])?;
        }
        TypeExpression::ReferenceOf {
            mutability,
            variance,
            target_type,
        } => {
            write!(f, [token("&")])?;

            if *mutability == Some(Mutability::Immutable) {
                write!(f, [Keyword::Readonly, space()])?;
            }

            if let Some(variance) = variance {
                match variance {
                    VarianceBound::Implements => write!(f, [Keyword::Implements, space()])?,
                    VarianceBound::Extends => write!(f, [Keyword::Extends, space()])?,
                    VarianceBound::Super => write!(f, [Keyword::Super, space()])?,
                }
            }

            write!(f, [target_type])?;
        }
        TypeExpression::PointerOf {
            mutability,
            target_type,
        } => {
            write!(f, [token("*")])?;

            if *mutability == Some(Mutability::Immutable) {
                write!(f, [Keyword::Readonly, space()])?;
            }

            write!(f, [target_type])?;
        }
        TypeExpression::Union { elements } => {
            write_union_type(f, node_id, elements, true)?;
        }
        TypeExpression::Intersection { elements } => {
            write!(
                f,
                [separated_entries(
                    "&",
                    elements,
                    TrailingSeparator::Omit,
                    None
                )]
            )?;
        }
        TypeExpression::Conditional {
            left,
            extends_type,
            then_type,
            else_type,
        } => {
            write!(
                f,
                [group(&format_args![
                    left,
                    space(),
                    Keyword::Extends,
                    space(),
                    extends_type,
                    indent(&format_args![
                        soft_line_break_or_space(),
                        token("?"),
                        space(),
                        then_type,
                        soft_line_break_or_space(),
                        token(":"),
                        space(),
                        else_type
                    ])
                ])]
            )?;
        }
        TypeExpression::Mapped {
            parameter,
            readonly,
            optional,
            value,
        } => {
            let span = f.context().span(node_id);
            let should_expand = f
                .context()
                .source_text()
                .has_newline_after_opening_brace(span.start);

            // mapped body
            let format_inner = format_with(|f| {
                write_mapped_modifier_prefix(f, *readonly, "readonly")?;

                let format_key = format_with(|f| {
                    write!(
                        f,
                        [
                            token("["),
                            parameter.name,
                            space(),
                            Keyword::In,
                            space(),
                            parameter.source_type
                        ]
                    )?;

                    if let Some(key_remap) = parameter.key_remap {
                        write!(f, [space(), Keyword::As, space(), key_remap])?;
                    }

                    write!(f, [token("]")])?;
                    write_mapped_modifier_suffix(f, *optional)
                });

                write!(f, [group(&format_key)])?;
                write!(f, [token(":"), space(), *value])?;
                write!(f, [if_group_breaks(&token(";"))])?;

                Ok(())
            });

            // grouped shell
            write!(
                f,
                [group(&format_args![
                    token("{"),
                    soft_block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        write!(f, [format_inner])?;

                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        Ok(())
                    })),
                    token("}")
                ])
                .should_expand(should_expand)]
            )?;
        }
        TypeExpression::Index { left, index } => {
            write_postfix_type_operand(f, *left)?;
            write!(f, [token("["), index, token("]")])?;
        }
        TypeExpression::TemplateLiteral { strings, spans } => {
            format_type_template_literal(node_id, strings, spans, f)?;
        }
        TypeExpression::Infer { name, constraint } => {
            write!(f, [Keyword::Infer, space(), *name])?;

            if let Some(constraint) = constraint {
                write!(f, [space(), Keyword::Extends, space(), constraint])?;
            }
        }
        TypeExpression::Predicate {
            asserts,
            subject,
            target,
        } => {
            if *asserts {
                write!(f, [Keyword::Asserts, space()])?;
            }

            match subject {
                TypePredicateSubject::Identifier(name) => write!(f, [*name])?,
                TypePredicateSubject::This => write!(f, [Keyword::This])?,
            }

            if let Some(target) = target {
                write!(f, [space(), Keyword::Is, space(), target])?;
            }
        }
        TypeExpression::Missing => {}
        TypeExpression::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, TypeExpression> for TypeExpression {
    fn format_node(
        &self,
        node_id: LocalNodeId<TypeExpression>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // prefix annotations
        write_type_expression_prefix_annotations(f, node_id, self)?;

        // body
        write_type_expression_body(f, node_id, self)?;

        // trailing annotations
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}

impl<'ast> FormatNode<'ast, TypeMember> for TypeMember {
    fn format_node(
        &self,
        node_id: LocalNodeId<TypeMember>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // prefix annotations
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // body
        match self {
            TypeMember::Field {
                is_optional,
                is_readonly,
                key,
                declared_type,
                ..
            } => {
                if *is_readonly {
                    write!(f, [Keyword::Readonly, space()])?;
                }

                write!(f, [key])?;

                if *is_optional {
                    write!(f, [token("?")])?;
                }

                write!(f, [token(":"), space(), declared_type])?;
            }
            TypeMember::Method {
                is_optional,
                key,
                signature,
                body: _,
                ..
            } => {
                write_type_signature(f, signature, *key, *is_optional)?;
            }
            TypeMember::IndexSignature {
                is_optional,
                is_readonly,
                name,
                key_type,
                value_type,
                ..
            } => {
                if *is_readonly {
                    write!(f, [Keyword::Readonly, space()])?;
                }

                write!(
                    f,
                    [token("["), *name, token(":"), space(), key_type, token("]")]
                )?;

                if *is_optional {
                    write!(f, [token("?")])?;
                }

                write!(f, [token(":"), space(), value_type])?;
            }
            TypeMember::Embed { value, .. } => {
                write!(f, [token("..."), *value])?;
            }
            TypeMember::AssociatedType {
                name,
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } => {
                write!(f, [Keyword::Type, space(), *name])?;

                if !generic_parameters.is_empty() {
                    write_generic_parameter_list(
                        f,
                        generic_parameters,
                        default_generic_parameter_trailing_separator(f),
                    )?;
                }

                if !where_clauses.is_empty() {
                    format_where_clause_with_break(f, where_clauses)?;
                }

                if let Some(constraint) = constraint {
                    write!(f, [token(":"), space(), *constraint])?;
                }

                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), *value])?;
                }
            }
            TypeMember::AssociatedConst {
                name,
                declared_type,
                value,
                ..
            } => {
                write!(f, [Keyword::Const, space(), *name])?;

                if let Some(declared_type) = declared_type {
                    write!(f, [token(":"), space(), *declared_type])?;
                }

                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), *value])?;
                }
            }
            TypeMember::Error { .. } => {
                write!(f, [token("/* ERROR */")])?;
            }
        }

        // trailing annotations
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}

impl<'ast> FormatNode<'ast, GenericArgument> for GenericArgument {
    fn format_node(
        &self,
        node_id: LocalNodeId<GenericArgument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // prefix annotations
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // body
        match self {
            GenericArgument::Type { value } => {
                write!(f, [value])?;
            }
            GenericArgument::Value { value } => {
                write!(f, [value])?;
            }
            GenericArgument::Error => {
                write!(f, [token("/* ERROR */")])?;
            }
        }

        // trailing annotations
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}

impl<'ast> FormatNode<'ast, TupleElement> for TupleElement {
    fn format_node(
        &self,
        node_id: LocalNodeId<TupleElement>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // prefix annotations
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // body
        match self {
            TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            } => {
                if *is_readonly {
                    write!(f, [Keyword::Readonly, space()])?;
                }

                if let Some(label) = label {
                    write!(f, [*label, token(":"), space()])?;
                }

                write!(f, [value])?;

                if *is_optional {
                    write!(f, [token("?")])?;
                }
            }
            TupleElement::Spread { label, value } => {
                if let Some(label) = label {
                    write!(f, [*label, token(":"), space()])?;
                }

                write!(f, [token("..."), value])?;
            }
            TupleElement::Error => {
                write!(f, [token("/* ERROR */")])?;
            }
        }

        // trailing annotations
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}
