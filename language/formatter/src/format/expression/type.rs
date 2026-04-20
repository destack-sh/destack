use super::conditional::ConditionalLayout;
use crate::format::annotation::{
    FormatLeadingComments, FormatTrailingComments, format_trailing_comments,
    infix_or_postfix_annotations, prefix_annotations, prefix_annotations_without_comments,
};
use crate::format::collection::literal::format_scalar_literal;
use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::format::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    parameter_is_variadic, should_hug_function_parameters, write_function_header_prefix,
    write_generic_parameter_list, write_signature_hug_parameter_list,
    write_signature_parameter_list, write_signature_return_type,
};
use crate::format::expression::format_type_template_literal;
use crate::format::operator::{
    format_generic_argument_list, write_colon_prefixed_type_annotation,
    write_type_annotation_prefix, write_type_expression_with_inline_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Comment, Declaration, Expression, FunctionSignature, GenericArgument, Key, Keyword,
    LocalNodeId, Mutability, NodeType, TokenType, TupleElement, TypeExpression, TypeMember,
    TypeModifier, TypePredicateSubject, VarianceBound,
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

/// One summary of union-leading comments.
#[derive(Debug, Clone, Copy, Default)]
struct LeadingCommentsInfo {
    has_own_line_comment: bool,
    has_end_of_line_comment: bool,
    has_trailing_own_line_block_comment: bool,
}

impl LeadingCommentsInfo {
    /// Build one leading-comment summary from comments.
    fn from_comment_nodes(comments: &[Comment]) -> Self {
        let mut info = Self::default();

        for comment in comments.iter().copied() {
            info.has_own_line_comment |= comment.preceded_by_newline();
            info.has_end_of_line_comment |= comment.followed_by_newline();
            info.has_trailing_own_line_block_comment |=
                comment.is_block() && comment.is_trailing() && comment.followed_by_newline();
        }

        info
    }
}

/// Return leading-comment info normalized for one union head.
fn union_leading_comment_info(comments: &[Comment]) -> LeadingCommentsInfo {
    LeadingCommentsInfo::from_comment_nodes(comments)
}

/// Write positional leading comments that belong directly before one type node.
pub(crate) fn write_type_expression_leading_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
) -> FormatResult<()> {
    if type_expression_body_owns_leading_comments(f, expression) {
        return Ok(());
    }

    let token_start = f.context().node_token_start(node_id);
    let comments = {
        let comments = f.context().comments();
        comments.comments_before(token_start).to_vec()
    };

    if comments.is_empty() {
        return Ok(());
    }

    write!(f, [FormatLeadingComments::Comments(&comments)])
}

/// Return the conditional layout for one type expression.
fn type_conditional_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> ConditionalLayout {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return ConditionalLayout::Root;
    };

    if parent_type != NodeType::TypeExpression {
        return ConditionalLayout::Root;
    }

    let parent_id = LocalNodeId::<TypeExpression>::new(parent_id);
    let TypeExpression::Conditional {
        left,
        extends_type,
        then_type,
        else_type,
    } = context.tree.get(parent_id)
    else {
        return ConditionalLayout::Root;
    };

    if *left == node_id || *extends_type == node_id {
        ConditionalLayout::NestedTest
    } else if *then_type == node_id {
        ConditionalLayout::NestedConsequent
    } else if *else_type == node_id {
        ConditionalLayout::NestedAlternate
    } else {
        ConditionalLayout::Root
    }
}

/// Return whether one type expression is a conditional.
fn type_expression_is_conditional(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        TypeExpression::Conditional { .. }
    )
}

/// Write the test shell of one conditional type.
fn write_type_conditional_test<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    layout: ConditionalLayout,
    left: LocalNodeId<TypeExpression>,
    extends_type: LocalNodeId<TypeExpression>,
    then_type: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let format_test = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [left, space(), Keyword::Extends, space(), extends_type])?;

        let trailing_comments = {
            let comments = f.context().comments();
            comments
                .comments_before_character(f.context().span(extends_type).end, b'?')
                .to_vec()
        };

        if !trailing_comments.is_empty() {
            write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
        }

        // own-line comments between `?` and the true arm need to stay with the true arm
        if f.context()
            .comments()
            .has_leading_own_line_comment(f.context().span(then_type).start)
        {
            let leading_comments = {
                let comments = f.context().comments();
                comments
                    .comments_before(f.context().span(then_type).start)
                    .to_vec()
            };

            if !leading_comments.is_empty() {
                write!(f, [FormatTrailingComments::Comments(&leading_comments)])?;
            }
        }

        Ok(())
    });

    if layout.is_nested_alternate() {
        write!(f, [align(2, &format_test)])?;
    } else {
        write!(f, [format_test])?;
    }

    Ok(())
}

/// Write the `? ... : ...` tail of one conditional type.
fn write_type_conditional_tail<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    then_type: LocalNodeId<TypeExpression>,
    else_type: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let then_leading_comments = f
        .context()
        .comments_after_previous_non_trivia_token_for(then_type);
    let else_leading_comments = f
        .context()
        .comments_after_previous_non_trivia_token_for(else_type);

    let format_then_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !then_leading_comments.is_empty() {
            write!(f, [FormatLeadingComments::Comments(&then_leading_comments)])?;
        }

        write_type_expression_without_prefix_annotations(f, then_type)?;

        let trailing_comments = {
            let comments = f.context().comments();
            comments
                .comments_before_character(f.context().span(then_type).end, b':')
                .to_vec()
        };

        if !trailing_comments.is_empty() {
            write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
        }

        Ok(())
    });

    let format_then_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let format_then_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if f.options().indent_style.is_space() {
                write!(f, [align(2, &format_then_type)])?;
            } else {
                write!(f, [indent(&format_then_type)])?;
            }

            Ok(())
        });

        if type_expression_is_conditional(f.context(), then_type) {
            write!(
                f,
                [
                    if_group_fits_on_line(&token("(")),
                    format_then_type,
                    if_group_fits_on_line(&token(")"))
                ]
            )?;
        } else {
            write!(f, [format_then_type])?;
        }

        Ok(())
    });

    let format_else_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !else_leading_comments.is_empty() {
            write!(f, [FormatLeadingComments::Comments(&else_leading_comments)])?;
        }

        write_type_expression_without_prefix_annotations(f, else_type)
    });

    let format_else_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if f.options().indent_style.is_space() {
            write!(f, [align(2, &format_else_type)])?;
        } else {
            write!(f, [indent(&format_else_type)])?;
        }

        Ok(())
    });

    write!(
        f,
        [format_args![
            soft_line_break_or_space(),
            token("?"),
            space(),
            format_then_type,
            soft_line_break_or_space(),
            token(":"),
            space(),
            format_else_type
        ]]
    )?;

    Ok(())
}

/// Format one conditional type with upstream-shaped nested layout.
fn write_conditional_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    left: LocalNodeId<TypeExpression>,
    extends_type: LocalNodeId<TypeExpression>,
    then_type: LocalNodeId<TypeExpression>,
    else_type: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let layout = type_conditional_layout(f.context(), node_id);

    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_conditional_test(f, layout, left, extends_type, then_type)?;

        let format_tail = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write_type_conditional_tail(f, then_type, else_type)
        });

        match layout {
            ConditionalLayout::Root | ConditionalLayout::NestedTest => {
                write!(f, [indent(&format_tail)])?;
            }
            ConditionalLayout::NestedConsequent => {
                write!(f, [dedent(&indent(&format_tail))])?;
            }
            ConditionalLayout::NestedAlternate => {
                write!(f, [format_tail])?;
            }
        }

        Ok(())
    });

    let grouped = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if layout.groups_at_root() {
            write!(f, [group(&format_inner)])?;
        } else {
            write!(f, [format_inner])?;
        }

        Ok(())
    });

    if layout.is_nested_test() {
        write!(f, [group(&soft_block_indent(&grouped))])?;
    } else {
        write!(f, [grouped])?;
    }

    Ok(())
}

/// Return the key token start for one mapped type when it can be located.
fn mapped_type_key_start(
    context: &DestackFormatContext<'_>,
    span: destack_source::Span,
) -> Option<u32> {
    let tokens = context.non_trivia_tokens_in_span(span);
    let mut saw_open_bracket = false;

    for token in tokens {
        if !saw_open_bracket {
            if token.token.ty == TokenType::OpenBracket {
                saw_open_bracket = true;
            }

            continue;
        }

        return Some(token.span.start);
    }

    None
}

/// Return comments after one mapped opening brace.
fn mapped_type_leading_body_comments(
    context: &DestackFormatContext<'_>,
    span: destack_source::Span,
) -> Vec<Comment> {
    let comments = context.comments();
    let key_start = mapped_type_key_start(context, span);

    if key_start.is_some_and(|key_start| comments.has_leading_own_line_comment(key_start)) {
        return key_start
            .map(|key_start| comments.comments_before(key_start).to_vec())
            .unwrap_or_default();
    }

    comments
        .comments_before_character(span.start, b'[')
        .to_vec()
}

/// Return one sorted and deduplicated comment slice.
fn normalize_comment_slice(mut comments: Vec<Comment>) -> Vec<Comment> {
    comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
    comments.dedup_by_key(|comment| (comment.span.start, comment.span.end));
    comments
}

/// Split one mapped value separator comment slice into inline and trailing comments.
fn split_mapped_value_separator_comments(comments: Vec<Comment>) -> (Vec<Comment>, Vec<Comment>) {
    let mut inline_comments = Vec::new();
    let mut trailing_comments = Vec::new();

    for comment in comments {
        if comment.is_line() {
            trailing_comments.push(comment);
        } else {
            inline_comments.push(comment);
        }
    }

    (
        normalize_comment_slice(inline_comments),
        normalize_comment_slice(trailing_comments),
    )
}

/// Return normalized comments between one mapped separator and its value.
fn mapped_type_value_separator_comments(
    context: &DestackFormatContext<'_>,
    value: LocalNodeId<TypeExpression>,
) -> Vec<Comment> {
    let value_span = context.span(value);
    let separator_start = context
        .previous_non_trivia_token_before_span(value_span)
        .map_or(value_span.start, |token| token.span.end);
    let comments = context
        .comments()
        .comments_in_range(separator_start, value_span.start)
        .to_vec();

    normalize_comment_slice(comments)
}

/// Return whether one type is object-like for intersection layout.
fn intersection_type_is_object_like(expression: &TypeExpression) -> bool {
    matches!(
        expression,
        TypeExpression::Object { .. } | TypeExpression::Mapped { .. }
    )
}

/// Write one intersection type with OXC-like object-chain layout.
fn write_intersection_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
) -> FormatResult<()> {
    // one annotated intersection keeps its explicit leading `&`
    if elements.len() == 1 && f.context().has_prefix_annotation(node_id) {
        return write!(f, [token("&"), elements[0]]);
    }

    let format_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let last_index = elements.len().saturating_sub(1);
        let mut previous_is_object_like = false;
        let mut is_chain_indented = false;

        for (index, element_id) in elements.iter().copied().enumerate() {
            let element = f.context().tree.get(element_id);
            let is_object_like = intersection_type_is_object_like(element);

            // first element stays inline
            if index == 0 {
                write!(f, [element_id])?;
            }
            // non-object edges use the standard breakable layout
            else if !(previous_is_object_like || is_object_like)
                || f.context()
                    .comments()
                    .has_leading_own_line_comment(f.context().span(element_id).start)
            {
                let content =
                    format_with(|f: &mut DestackFormatter<'ast, '_>| write!(f, [element_id]));

                write!(f, [soft_line_indent_or_space(&content)])?;
            }
            // object-like chains stay tighter, with one indentation step on mixed chains
            else {
                write!(f, [space()])?;

                if !previous_is_object_like || !is_object_like {
                    is_chain_indented = index > 1;
                }

                if is_chain_indented {
                    write!(f, [indent(&element_id)])?;
                } else {
                    write!(f, [element_id])?;
                }
            }

            // separator
            if index < last_index {
                write!(f, [space(), token("&")])?;
            }

            previous_is_object_like = is_object_like;
        }

        Ok(())
    });

    write!(f, [group(&format_content)])
}

/// Return whether a type object body starts on its own line in source.
fn type_object_members_have_leading_newline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    members: &[LocalNodeId<TypeMember>],
) -> bool {
    let Some(first_member_id) = members.first().copied() else {
        return false;
    };

    let object_span = context.span(node_id);
    let first_member_span = context.span(first_member_id);

    if object_span.file != first_member_span.file || object_span.start >= first_member_span.start {
        return false;
    }

    context.has_newline(destack_source::Span::new(
        object_span.file,
        object_span.start,
        first_member_span.start,
    ))
}

/// Flatten nested union wrappers into one operand list.
fn flatten_union_elements(
    context: &DestackFormatter<'_, '_>,
    expression_id: LocalNodeId<TypeExpression>,
    elements: &mut Vec<LocalNodeId<TypeExpression>>,
) {
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
) -> bool {
    if elements.len() <= 1 {
        return true;
    }

    let has_object_type = elements.iter().copied().any(|element_id| {
        matches!(
            f.context().tree.get(element_id),
            TypeExpression::Object { .. } | TypeExpression::Reference { .. }
        )
    });

    if !has_object_type {
        return false;
    }

    let nullish_count = elements
        .iter()
        .copied()
        .filter(|element_id| {
            matches!(
                f.context().tree.get(*element_id),
                TypeExpression::Literal {
                    value: destack_ast::TypeLiteral::Void | destack_ast::TypeLiteral::Null,
                }
            )
        })
        .count();

    if elements.len() - 1 != nullish_count {
        return false;
    }

    let mut start = f.context().span(node_id).start;

    for element_id in elements.iter().copied() {
        let element_span = f.context().span(element_id);

        if f.context()
            .comments()
            .has_comment_in_range(start, element_span.start)
        {
            return false;
        }

        start = element_span.end;
    }

    true
}

/// Return whether one multiline union should own one extra indent.
fn union_should_indent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    leading_comment_info: LeadingCommentsInfo,
) -> bool {
    let Some((parent_id, parent_type)) = union_indent_parent(context, node_id) else {
        return false;
    };

    match parent_type {
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(parent_id);

            match context.tree.get(declaration_id) {
                // type aliases have one comment-sensitive shell
                Declaration::Type(declaration) => type_alias_union_should_indent(
                    context,
                    declaration_id,
                    declaration,
                    leading_comment_info,
                ),

                // other declarations follow the default union shell
                _ => true,
            }
        }

        // tuple slots already indent their element body
        NodeType::TupleElement => false,

        // direct type arguments already indent their value wrapper
        NodeType::GenericArgument => false,

        // expression parents need one more shape-based split
        NodeType::Expression => {
            let expression_id = LocalNodeId::<Expression>::new(parent_id);

            union_expression_should_indent(context, expression_id)
        }

        // other parents use the default union shell
        _ => true,
    }
}

/// Return the last non-trivia token end before one type declaration value.
fn type_declaration_head_end(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
    declaration: &destack_ast::TypeDeclaration,
) -> u32 {
    let declaration_span = context.span(declaration_id);
    let value_start = context.span(declaration.value).start;

    context
        .tokens
        .iter()
        .copied()
        .chain(context.side_tokens.iter().copied())
        .filter(|token| {
            token.span.file == declaration_span.file
                && token.span.start >= declaration_span.start
                && token.span.end <= value_start
                && !matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline)
        })
        .map(|token| token.span.end)
        .max()
        .unwrap_or(declaration_span.start)
}

/// Return whether one type-alias union should keep its extra union indent shell.
fn type_alias_union_should_indent(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
    declaration: &destack_ast::TypeDeclaration,
    leading_comment_info: LeadingCommentsInfo,
) -> bool {
    let _ = leading_comment_info;

    let head_end = type_declaration_head_end(context, declaration_id, declaration);

    !context
        .comments()
        .printed_comments()
        .last()
        .is_some_and(|comment| comment.span.start > head_end && comment.followed_by_newline())
}

/// Return the outer parent that decides one union indent shell.
fn union_indent_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> Option<(u32, NodeType)> {
    let (parent_id, parent_type, _) = effective_type_parent_slot(context, node_id)?;

    Some((parent_id, parent_type))
}

/// Return whether one expression parent should add one union indent shell.
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

        // other expression parents use the default union shell
        _ => true,
    }
}

/// Return whether one union is the only type in its union chain.
fn union_is_only_type_in_chain(
    context: &DestackFormatContext<'_>,
    mut node_id: LocalNodeId<TypeExpression>,
    elements_len: usize,
) -> bool {
    let mut only_type = elements_len == 1;

    loop {
        let Some((parent_id, parent_type)) = context.parent(node_id) else {
            return only_type;
        };
        if parent_type != NodeType::TypeExpression {
            return only_type;
        }

        let parent_id = LocalNodeId::<TypeExpression>::new(parent_id);
        match context.tree.get(parent_id) {
            // singleton union wrappers decide the outer grouping role
            TypeExpression::Union { elements } if elements.len() == 1 => {
                only_type = true;
                node_id = parent_id;
            }

            _ => return only_type,
        }
    }
}

/// Write one inline union body without multiline grouping logic.
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
            write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
        }
    }

    Ok(())
}

/// Write one union type with break-aware leading separators.
fn write_union_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
    is_in_explicit_parentheses: bool,
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

    // leading comments
    let union_leading_comments = {
        let comments = f.context().comments();
        comments
            .comments_before(f.context().node_token_start(node_id))
            .to_vec()
    };

    // inline unions
    let leading_comment_info = union_leading_comment_info(&union_leading_comments);
    let should_hug = union_should_hug(f, node_id, &flattened_elements);

    if should_hug {
        return write_inline_union_type(f, &flattened_elements);
    }

    // multiline indent
    let should_indent = union_should_indent(f.context(), node_id, leading_comment_info);
    let needs_parentheses = !is_in_explicit_parentheses
        && type_expression_needs_parentheses_in_parent(f.context(), node_id);
    let only_type = union_is_only_type_in_chain(f.context(), node_id, flattened_elements.len());
    let content_group_id = f.group_id("union_type");

    // grouped content
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let leading_separator = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if should_indent && union_leading_comments.is_empty() {
                write!(f, [soft_line_break_or_space()])?;
            }

            write!(f, [token("|"), space()])?;

            Ok(())
        });

        write!(
            f,
            [if_group_breaks(&leading_separator).with_group_id(Some(content_group_id))]
        )?;

        let mut element_iter = flattened_elements.iter().copied().peekable();

        while let Some(element_id) = element_iter.next() {
            // operand
            if should_hug {
                write!(f, [element_id])?;
            } else {
                write!(f, [align(2, &element_id)])?;
            }

            // following separators
            if let Some(next_element_id) = element_iter.peek().copied() {
                let current_element_span = f.context().span(element_id);
                let next_element_span = f.context().span(next_element_id);

                let comments_before_separator = {
                    let comments = f.context().comments();
                    comments
                        .comments_before_character(current_element_span.end, b'|')
                        .to_vec()
                };
                if !comments_before_separator.is_empty() {
                    write!(
                        f,
                        [FormatTrailingComments::Comments(&comments_before_separator)]
                    )?;
                }

                if f.context()
                    .comments()
                    .has_leading_own_line_comment(next_element_span.start)
                {
                    let comments_before_next_element = {
                        let comments = f.context().comments();
                        comments.comments_before(next_element_span.start).to_vec()
                    };

                    if !comments_before_next_element.is_empty() {
                        write!(
                            f,
                            [FormatTrailingComments::Comments(
                                &comments_before_next_element
                            )]
                        )?;
                    }
                }

                if should_hug {
                    write!(f, [space()])?;
                } else {
                    write!(f, [soft_line_break_or_space()])?;
                }

                write!(f, [token("|"), space()])?;
            }
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
                write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
            }
        }

        Ok(())
    });

    let format_inner_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let has_own_line_comment = leading_comment_info.has_own_line_comment
            || matches!(
                union_indent_parent(f.context(), node_id),
                Some((parent_id, NodeType::Declaration))
                    if matches!(
                        f.context()
                            .tree
                            .get(LocalNodeId::<Declaration>::new(parent_id)),
                        Declaration::Type(_)
                    )
            ) && leading_comment_info.has_trailing_own_line_block_comment;

        if has_own_line_comment && !only_type {
            write!(f, [hard_line_break()])?;
        } else if leading_comment_info.has_end_of_line_comment && only_type {
            write!(f, [soft_line_break()])?;
        }

        if !union_leading_comments.is_empty() {
            write!(
                f,
                [FormatLeadingComments::Comments(&union_leading_comments)]
            )?;
        }

        if !leading_comment_info.has_end_of_line_comment && has_own_line_comment && only_type {
            write!(f, [soft_line_break()])?;
        }

        write!(f, [group(&content).with_id(Some(content_group_id))])
    });

    if should_indent && !needs_parentheses {
        write!(f, [group(&indent(&format_inner_content))])
    } else {
        write!(f, [group(&format_inner_content)])
    }
}

/// Return whether one type body formats its own leading comments.
fn type_expression_body_owns_leading_comments(
    f: &DestackFormatter<'_, '_>,
    expression: &TypeExpression,
) -> bool {
    let _ = f;

    matches!(expression, TypeExpression::Union { .. })
}

/// Return trailing-comment bounds for one type expression.
fn type_expression_trailing_comment_bounds(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> Option<(destack_source::Span, u32)> {
    let (parent_id, parent_type) = context.parent(node_id)?;
    let parent_span = context.span_by_id(parent_id);
    let node_span = context.span(node_id);
    let following_span_start = if parent_type == NodeType::TypeExpression {
        let parent_id = LocalNodeId::<TypeExpression>::new(parent_id);

        if matches!(
            context.tree.get(parent_id),
            TypeExpression::Mapped { value, .. } if *value == node_id
        ) {
            type_expression_trailing_comment_bounds(context, parent_id)
                .map_or(0, |(_, start)| start)
        } else {
            context
                .next_non_trivia_token_after_span(node_span)
                .filter(|token| token.span.file == parent_span.file)
                .filter(|token| token.span.start > node_span.end)
                .filter(|token| token.span.start < parent_span.end)
                .map_or(0, |token| token.span.start)
        }
    } else {
        context
            .next_non_trivia_token_after_span(node_span)
            .filter(|token| token.span.file == parent_span.file)
            .filter(|token| token.span.start > node_span.end)
            .filter(|token| token.span.start < parent_span.end)
            .map_or(0, |token| token.span.start)
    };

    Some((parent_span, following_span_start))
}

/// Write prefix annotations for one type expression.
pub(crate) fn write_type_expression_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
) -> FormatResult<()> {
    let _ = expression;

    write!(
        f,
        [prefix_annotations_without_comments(f.context(), node_id)]
    )
}

/// Write one type expression without emitting prefix annotations.
pub(crate) fn write_type_expression_without_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(node_id);
    write_type_expression_body(f, node_id, expression, false)?;
    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
}

/// Write one type expression node with optional derived parentheses.
fn write_type_expression_node<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
    allow_derived_parentheses: bool,
) -> FormatResult<()> {
    let needs_parentheses = allow_derived_parentheses
        && type_expression_needs_parentheses_in_parent(f.context(), node_id);

    // prefix annotations
    write_type_expression_prefix_annotations(f, node_id, expression)?;

    // positional leading comments
    write_type_expression_leading_comments(f, node_id, expression)?;

    // derived parentheses
    if needs_parentheses {
        write!(f, [token("(")])?;
    }

    // body
    write_type_expression_body(f, node_id, expression, !allow_derived_parentheses)?;

    // trailing comments
    if let Some((enclosing_span, following_span_start)) =
        type_expression_trailing_comment_bounds(f.context(), node_id)
    {
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                f.context().span(node_id),
                following_span_start
            )]
        )?;
    }

    // derived parentheses
    if needs_parentheses {
        write!(f, [token(")")])?;
    }

    // trailing annotations
    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
}

/// Write one type with parentheses when one postfix operator needs them.
fn write_postfix_type_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let expression_id = normalize_postfix_type_operand(f.context(), expression_id);

    // postfix grouping
    if type_needs_postfix_parentheses(f.context(), expression_id) {
        let expression = f.context().tree.get(expression_id);

        write!(
            f,
            [group(&format_args![
                token("("),
                format_with(|f| write_type_expression_node(f, expression_id, expression, false)),
                soft_line_break(),
                token(")")
            ])]
        )
    } else {
        write!(f, [expression_id])
    }
}

/// Return one effective parent plus the outermost transparent child in its slot.
fn effective_type_parent_slot(
    context: &DestackFormatContext<'_>,
    mut node_id: LocalNodeId<TypeExpression>,
) -> Option<(u32, NodeType, LocalNodeId<TypeExpression>)> {
    let mut parent_slot_type_id = node_id;

    loop {
        let (parent_id, parent_type) = context.parent(node_id)?;

        if parent_type != NodeType::TypeExpression {
            return Some((parent_id, parent_type, parent_slot_type_id));
        }

        let parent_type_id = LocalNodeId::<TypeExpression>::new(parent_id);
        match context.tree.get(parent_type_id) {
            // single-member unions and intersections are also transparent here
            TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                if elements.len() <= 1 && elements.first().copied() == Some(node_id) {
                    parent_slot_type_id = parent_type_id;
                    node_id = parent_type_id;
                } else {
                    return Some((parent_id, parent_type, parent_slot_type_id));
                }
            }

            // other type parents decide precedence directly
            _ => return Some((parent_id, parent_type, parent_slot_type_id)),
        }
    }
}

/// Return whether one parent type forces parentheses around its child position.
fn type_parent_requires_parentheses(
    context: &DestackFormatContext<'_>,
    parent_id: LocalNodeId<TypeExpression>,
    child_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(parent_id) {
        // postfix and indexed object positions need explicit grouping
        TypeExpression::Array { element } => *element == child_id,
        TypeExpression::Index { left, .. } => *left == child_id,

        // unary type operators bind tighter than unions, intersections, and conditionals
        TypeExpression::Readonly { target_type }
        | TypeExpression::KeyOf { target_type }
        | TypeExpression::Must { target_type }
        | TypeExpression::AsComptime { target_type }
        | TypeExpression::Not { target_type }
        | TypeExpression::ValueOf { target_type, .. }
        | TypeExpression::ReferenceOf { target_type, .. }
        | TypeExpression::PointerOf { target_type, .. } => *target_type == child_id,

        // value-space typeof keeps its own precedence shell
        TypeExpression::TypeOfValue { .. } => false,

        _ => false,
    }
}

/// Return whether one type expression needs derived parentheses in its effective parent.
fn type_expression_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> bool {
    let Some((parent_id, parent_type, parent_slot_type_id)) =
        effective_type_parent_slot(context, node_id)
    else {
        return false;
    };

    if parent_type != NodeType::TypeExpression {
        return false;
    }

    let parent_id = LocalNodeId::<TypeExpression>::new(parent_id);
    match context.tree.get(node_id) {
        TypeExpression::Union { elements } => {
            if elements.len() <= 1 {
                return false;
            }

            match context.tree.get(parent_id) {
                TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                    elements.len() > 1
                }
                _ => type_parent_requires_parentheses(context, parent_id, parent_slot_type_id),
            }
        }
        TypeExpression::Intersection { elements } => {
            if elements.len() <= 1 {
                return false;
            }

            match context.tree.get(parent_id) {
                TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                    elements.len() > 1
                }
                _ => type_parent_requires_parentheses(context, parent_id, parent_slot_type_id),
            }
        }
        TypeExpression::Conditional { .. } => match context.tree.get(parent_id) {
            TypeExpression::Conditional {
                left, extends_type, ..
            } => *left == parent_slot_type_id || *extends_type == parent_slot_type_id,
            TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                elements.len() > 1
            }
            _ => type_parent_requires_parentheses(context, parent_id, parent_slot_type_id),
        },
        TypeExpression::Readonly { .. }
        | TypeExpression::KeyOf { .. }
        | TypeExpression::TypeOfValue { .. }
        | TypeExpression::Must { .. }
        | TypeExpression::AsComptime { .. }
        | TypeExpression::Not { .. }
        | TypeExpression::ValueOf { .. }
        | TypeExpression::ReferenceOf { .. }
        | TypeExpression::PointerOf { .. } => {
            type_parent_requires_parentheses(context, parent_id, parent_slot_type_id)
        }
        _ => false,
    }
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

    // simple rest parameters should stay grouped with the return type
    if parameters.len() == 1 && parameter_is_variadic(f.context(), parameters[0]) {
        return write_signature_hug_parameter_list(f, &parameters);
    }

    // hugging
    if should_hug_function_parameters(f.context(), &parameters, false) {
        return write_signature_hug_parameter_list(f, &parameters);
    }

    // grouped list
    let disallow_trailing_parameter_separator = parameters
        .last()
        .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id));

    write_signature_parameter_list(f, &parameters, disallow_trailing_parameter_separator)
}

/// Write one type-space function signature.
fn write_type_signature<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeMember>,
    signature: &FunctionSignature,
    key: Option<Key>,
    is_optional: bool,
) -> FormatResult<()> {
    let has_name_or_key = key.is_some();

    let signature_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
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
            write_signature_return_type(f, node_id, return_type)?;
        }

        // where clauses
        if !signature.where_clauses.is_empty() {
            format_where_clause_with_break(f, &signature.where_clauses)?;
        }

        Ok(())
    });

    write!(f, [group(&signature_content)])
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

/// Format one type-member list with group-aware separators.
pub(crate) fn format_type_member_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<TypeMember>],
) -> FormatResult<()> {
    write!(
        f,
        [separated_entries(
            ";",
            members,
            TrailingSeparator::Allowed,
            None
        )]
    )
}

/// Write one type body without prefix annotations.
fn write_type_expression_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
    is_in_explicit_parentheses: bool,
) -> FormatResult<()> {
    match expression {
        TypeExpression::Parenthesized { .. } => {
            unreachable!("formatter normalization should remove parenthesized type expressions");
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
            let should_expand =
                type_object_members_have_leading_newline(f.context(), node_id, members);

            write!(
                f,
                [group(&format_args![
                    token("{"),
                    soft_block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        format_type_member_list(f, members)?;

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
        TypeExpression::Declaration { declaration } => {
            write!(f, [declaration])?;
        }
        TypeExpression::Reference {
            path,
            generic_arguments,
        } => {
            write!(f, [path])?;

            if !generic_arguments.is_empty() {
                format_generic_argument_list(f, generic_arguments)?;
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
                format_generic_argument_list(f, generic_arguments)?;
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
                format_generic_argument_list(f, generic_arguments)?;
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
            write_union_type(f, node_id, elements, is_in_explicit_parentheses)?;
        }
        TypeExpression::Intersection { elements } => {
            write_intersection_type(f, node_id, elements)?;
        }
        TypeExpression::Conditional {
            left,
            extends_type,
            then_type,
            else_type,
        } => {
            write_conditional_type(f, node_id, *left, *extends_type, *then_type, *else_type)?;
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
            let leading_comments = mapped_type_leading_body_comments(f.context(), span);
            let separator_comments = mapped_type_value_separator_comments(f.context(), *value);
            let (inline_separator_comments, trailing_separator_comments) =
                split_mapped_value_separator_comments(separator_comments);

            // mapped body
            let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                // comments after `{`
                if should_expand && !leading_comments.is_empty() {
                    write!(f, [FormatLeadingComments::Comments(&leading_comments)])?;
                }

                // modifiers
                write_mapped_modifier_prefix(f, *readonly, "readonly")?;

                // key head
                let format_key = format_with(|f: &mut DestackFormatter<'ast, '_>| {
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

                // value
                write!(f, [group(&format_key)])?;
                write!(f, [token(":"), space()])?;

                if !inline_separator_comments.is_empty() {
                    write!(
                        f,
                        [FormatLeadingComments::Comments(&inline_separator_comments)]
                    )?;
                }

                if !trailing_separator_comments.is_empty() {
                    write!(
                        f,
                        [FormatTrailingComments::Comments(
                            &trailing_separator_comments
                        )]
                    )?;
                }

                write!(f, [*value])?;

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
        write_type_expression_node(f, node_id, self, true)
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

                if let Some(declared_type) = declared_type {
                    if let Some(type_span) =
                        f.context().tree.get_side_span(node_id, NodeSpanType::Type)
                    {
                        write_type_annotation_prefix(f, type_span.start)?;
                        write_type_expression_with_inline_prefix_annotations(f, *declared_type)?;
                    } else {
                        write_colon_prefixed_type_annotation(f, *declared_type)?;
                    }
                }
            }
            TypeMember::Method {
                is_optional,
                key,
                signature,
                body: _,
                ..
            } => {
                write_type_signature(f, node_id, signature, *key, *is_optional)?;
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
            TypeMember::Error => {
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
