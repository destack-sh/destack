use super::declarator::format_declarator;
use super::dispatch::format_expression;
use super::{
    format_expanded_ternary_expression, write_expression_without_prefix_annotations,
    write_expression_without_trailing_comments,
};
use crate::annotation::{
    DanglingIndentMode, FormatDanglingComments, FormatLeadingComments, FormatTrailingComments,
    block_infix_annotations, format_leading_comments, infix_or_postfix_annotations,
    postfix_annotations, prefix_annotations, prefix_comment_nodes, write_annotation_sequence,
    write_comment_slice,
};
use crate::chain::transparent_inner_expression;
use crate::declaration::sequence::block_statement_sequence;
use crate::declaration::signature::expression_body_requires_head_space;
use crate::declaration::statement::format_block;
use crate::declaration::{
    empty_block_with_infix_annotations, statement_wrapper_needs_semicolon,
    write_statement_terminator, write_statement_terminator_after_anchor,
};
use crate::expression::ExpressionLeftSide;
use crate::file::{node_has_ignore_directive, node_has_trailing_line_ignore_directive};
use crate::tree::tree_literal_should_break;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Asynchrony, Block, BlockFormat, DecoratorPosition, Expression, ForEachBinding,
    ForEachDeclarationKind, ForEachKind, IfCondition, IfKind, Keyword, LetKind, LocalNodeId,
    MatchCase, MatchKind, MatchSelector, Mutability, NodeType, Pattern, TokenType, TypeExpression,
    WhileKind, YieldCardinality,
};
use destack_core::StringId;
use destack_fir::format::{Buffer, Format, FormatError, FormatResult};
use destack_fir::prelude::{
    block_indent, empty_line, expand_parent, format_with, group, hard_line_break,
    line_suffix_boundary, soft_block_indent, soft_line_indent_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// Write one `if` or `while` test expression before the closing `)`.
fn write_if_or_while_test_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    condition_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_expression_without_trailing_comments(f, condition_id)?;

    let trailing_comments = {
        let comments = f.context().comments();
        comments
            .comments_before_character(f.context().span(condition_id).end, b')')
            .to_vec()
    };
    if trailing_comments.is_empty() {
        return Ok(());
    }

    write!(
        f,
        [
            space(),
            FormatTrailingComments::Comments(&trailing_comments)
        ]
    )
}

/// Format one grouped control head before the closing `)`.
fn write_grouped_control_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    head: &impl Format<DestackFormatContext<'ast>>,
) -> FormatResult<()> {
    write!(f, [group(&soft_block_indent(head))])
}

/// Write comments that belong to one empty statement body before its semicolon.
fn write_comments_for_empty_statement_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    body_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    if !is_empty_statement_block(f.context(), body_id) {
        return Ok(());
    }

    let comments = {
        let comments = f.context().comments();
        comments
            .comments_before(f.context().span(body_id).start)
            .to_vec()
    };
    if comments.is_empty() {
        return Ok(());
    }

    write_comment_slice(f, &comments)
}

/// Format one statement-body expression with statement-separator semantics.
fn format_statement_body_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    format_statement_body_expression_with_semicolon(f, expression_id, true)
}

/// Format one control-flow body expression with configurable semicolon handling.
fn format_statement_body_expression_with_semicolon<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    has_trailing_semicolon: bool,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);
    let is_ignored = node_has_ignore_directive(f.context(), expression_id);
    let expression_span = f.context().span(expression_id);
    let token_start = f.context().expression_token_start(expression_id);
    let token_start_span = Span::new(expression_span.file, token_start, token_start);

    write!(f, [format_leading_comments(token_start_span)])?;

    write!(f, [prefix_annotations(f.context(), expression_id)])?;
    format_expression(f, expression_id, expression, is_ignored)?;

    if has_trailing_semicolon && statement_wrapper_needs_semicolon(f.context(), expression_id) {
        write_statement_terminator(f, expression_id)?;
    }

    let if_chain_handles_annotations = matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        }
    );
    if !if_chain_handles_annotations {
        write!(
            f,
            [infix_or_postfix_annotations(f.context(), expression_id)]
        )?;
    }

    Ok(())
}

/// Write prefix items for one match case.
fn write_match_case_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    case_id: LocalNodeId<MatchCase>,
) -> FormatResult<()> {
    let leading_comments = prefix_comment_nodes(f.context(), case_id);
    if !leading_comments.is_empty() {
        write!(f, [FormatLeadingComments::Comments(&leading_comments)])?;
    }

    let prefix_annotation_ids: Vec<_> = f
        .context()
        .annotation_ids(case_id)
        .iter()
        .copied()
        .filter(|annotation_id| {
            matches!(
                f.context().annotation(*annotation_id).position,
                DecoratorPosition::LinePrefix | DecoratorPosition::BlockPrefix
            )
        })
        .collect();
    if prefix_annotation_ids.is_empty() {
        return Ok(());
    }

    write_annotation_sequence(f, &prefix_annotation_ids)
}

/// Format a statement body block, preserving wrapper semantics.
pub(crate) fn format_statement_body_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    let is_statement_wrapper = is_statement_wrapper_block(f.context(), block_id);
    if !is_statement_wrapper {
        write!(f, [block_id])?;
        return Ok(());
    }

    write!(f, [prefix_annotations(f.context(), block_id)])?;

    if block.is_empty() {
        write_statement_terminator_after_anchor(f, f.context().span(block_id).start)?;
    } else if block.len() == 1 {
        let expression_id = block.first_expression().expect("single-expression block");
        if expression_has_block_prefix_annotation(f.context(), expression_id) {
            write!(
                f,
                [
                    hard_line_break(),
                    group(&block_indent(&format_with(|f| {
                        format_statement_body_expression(f, expression_id)
                    })))
                ]
            )?;
        } else {
            format_statement_body_expression(f, expression_id)?;
        }
    } else {
        write!(f, [block_id])?;
    }

    write!(f, [infix_or_postfix_annotations(f.context(), block_id)])?;
    Ok(())
}

/// Format one block-backed statement body after a control-flow head.
fn format_statement_body_block_after_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    if !is_statement_wrapper_block(f.context(), block_id) {
        write!(f, [space()])?;
        return format_block(f, block_id);
    }

    if block.is_empty() {
        let has_leading_comments = {
            let comments = f.context().comments();
            !comments
                .comments_before(f.context().span(block_id).start)
                .is_empty()
        };

        if has_leading_comments {
            write!(f, [space()])?;
        }

        return format_statement_body_block(f, block_id);
    }

    if block.len() == 1 {
        let expression_id = block.first_expression().expect("single-expression block");
        let has_leading_comments = {
            let comments = f.context().comments();
            !comments
                .comments_before(f.context().span(expression_id).start)
                .is_empty()
        };
        let body = format_with(|f| format_statement_body_expression(f, expression_id));

        if expression_has_block_prefix_annotation(f.context(), expression_id)
            || has_leading_comments
        {
            return write!(f, [hard_line_break(), group(&block_indent(&body))]);
        }

        return write!(f, [soft_line_indent_or_space(&body)]);
    }

    write!(f, [space(), block_id])
}

/// Return true when this block originated from a statement wrapper instead of braces.
fn is_statement_wrapper_block<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    block.format == BlockFormat::Implicit
}

/// Return true when this block is an empty statement wrapper.
pub(crate) fn is_empty_statement_block<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    is_statement_wrapper_block(context, block_id) && block.is_empty()
}

/// Detect a source binding keyword for a for each pattern binding.
pub(crate) fn detect_for_each_binding_keyword<'ast>(
    context: &DestackFormatContext<'ast>,
    _for_each_id: LocalNodeId<Expression>,
    pattern_id: LocalNodeId<Pattern>,
) -> Option<Keyword> {
    let pattern_span = context.span(pattern_id);
    let keyword_token = context.previous_non_whitespace_token_before_span(pattern_span)?;
    if keyword_token.token.ty != TokenType::Identifier {
        return None;
    }

    if context
        .token_keyword(keyword_token)
        .is_some_and(|keyword| keyword == Keyword::Let)
    {
        Some(Keyword::Let)
    } else if context
        .token_keyword(keyword_token)
        .is_some_and(|keyword| keyword == Keyword::Const)
    {
        Some(Keyword::Const)
    } else if context
        .token_keyword(keyword_token)
        .is_some_and(|keyword| keyword == Keyword::Var)
    {
        Some(Keyword::Var)
    } else {
        None
    }
}

/// Format a for each binding pattern without repeating root mutability keywords.
pub(crate) fn format_for_each_binding_pattern<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern_id: LocalNodeId<Pattern>,
) -> FormatResult<()> {
    match f.context().tree.get(pattern_id) {
        Pattern::Binding { name, pattern, .. } => {
            write!(f, [name])?;
            if let Some(pattern) = pattern {
                write!(f, [token(":"), space(), pattern])?;
            }
            Ok(())
        }
        _ => write!(f, [pattern_id]),
    }
}

/// Return whether an if branch should include a space after the condition head.
fn expression_has_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let annotations = context.annotation_ids(expression_id);
    if annotations.is_empty() {
        return false;
    }

    annotations.iter().copied().any(|annotation_id| {
        if context.annotation(annotation_id).position != DecoratorPosition::BlockPrefix {
            return false;
        }

        let annotation_span = context.annotation_span(annotation_id);
        let is_multiline = annotation_span.start < annotation_span.end
            && !context
                .file
                .is_same_line(annotation_span.start, annotation_span.end.saturating_sub(1));

        is_multiline || context.annotation_starts_on_own_line(annotation_id)
    })
}

/// Return the single statement expression inside one transparent control-body wrapper.
fn transparent_control_body_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let Expression::Block(block_id) = context.tree.get(expression_id) else {
        return None;
    };

    let block = context.tree.get(*block_id);
    if block.format != BlockFormat::Implicit || block.len() != 1 {
        return None;
    }

    if context.has_annotation(expression_id) || context.has_annotation(*block_id) {
        return None;
    }

    block.first_expression()
}

/// Return the empty implicit statement wrapper behind one transparent control body.
fn transparent_empty_control_body(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Block>> {
    let Expression::Block(block_id) = context.tree.get(expression_id) else {
        return None;
    };

    let block = context.tree.get(*block_id);
    if block.format != BlockFormat::Implicit || !block.is_empty() {
        return None;
    }

    if context.has_annotation(expression_id) || context.has_annotation(*block_id) {
        return None;
    }

    Some(*block_id)
}

/// Format one transparent empty statement body after a control-flow head.
fn format_empty_statement_body_after_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    write_statement_terminator_after_anchor(f, f.context().span(block_id).start)
}

/// Return whether one expression is the consequent of an if with an alternate.
fn expression_is_if_consequent_with_alternate(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((block_id, NodeType::Block)) = context.parent(expression_id) else {
        return false;
    };
    let block_id = LocalNodeId::<Block>::new(block_id);

    let Some((block_expression_id, NodeType::Expression)) = context.parent(block_id) else {
        return false;
    };
    let block_expression_id = LocalNodeId::<Expression>::new(block_expression_id);

    let Some((if_expression_id, NodeType::Expression)) = context.parent(block_expression_id) else {
        return false;
    };
    let if_expression_id = LocalNodeId::<Expression>::new(if_expression_id);

    matches!(
        context.tree.get(if_expression_id),
        Expression::If {
            then_expression,
            else_expression: Some(_),
            ..
        } if *then_expression == block_expression_id
    )
}

/// Format one non-block statement body after a control-flow head.
fn format_statement_body_expression_after_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let has_leading_comments = {
        let comments = f.context().comments();
        !comments
            .comments_before(f.context().span(expression_id).start)
            .is_empty()
    };
    let body = format_with(|f| format_statement_body_expression(f, expression_id));

    let is_if_consequent_with_alternate =
        expression_is_if_consequent_with_alternate(f.context(), expression_id);
    let has_end_of_line_comment = f
        .context()
        .comments()
        .has_end_of_line_comment_after(f.context().span(expression_id).end);
    let has_trailing_ignore_directive =
        node_has_trailing_line_ignore_directive(f.context(), expression_id);
    if is_if_consequent_with_alternate && (has_end_of_line_comment || has_trailing_ignore_directive)
    {
        write!(f, [hard_line_break(), group(&block_indent(&body))])?;
        return Ok(());
    }

    if expression_has_block_prefix_annotation(f.context(), expression_id) || has_leading_comments {
        write!(f, [hard_line_break(), group(&block_indent(&body))])?;
        return Ok(());
    }

    write!(f, [soft_line_indent_or_space(&body)])
}

/// Return whether one adjacent argument is nested directly inside `yield`.
fn adjacent_statement_argument_is_inside_yield(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    ctx.parent(expression_id)
        .is_some_and(|(parent_id, parent_type)| {
            parent_type == NodeType::Expression
                && matches!(
                    ctx.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Yield { .. }
                )
        })
}

/// Return whether one member gap has own-line or multiline comments.
fn adjacent_statement_member_gap_has_comments(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let (left, property_start) = match ctx.tree.get(expression_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            let Some(property_span) = ctx.tree.get_main_span(expression_id) else {
                return false;
            };

            (*left, property_span.start)
        }
        _ => return false,
    };

    let left_span = ctx.span(left);
    if left_span.file != ctx.span(expression_id).file || property_start <= left_span.end {
        return false;
    }

    ctx.comments()
        .comments_in_range(left_span.end, property_start)
        .iter()
        .copied()
        .any(|comment| {
            (comment.is_block() && ctx.has_newline(comment.span)) || comment.preceded_by_newline()
        })
}

/// Return whether one adjacent statement argument has leading comments.
fn adjacent_statement_argument_has_leading_comments(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Expression>,
) -> bool {
    let is_inside_yield = adjacent_statement_argument_is_inside_yield(ctx, argument_id);
    let mut left_side = Some(ExpressionLeftSide::new(argument_id));

    while let Some(current_left_side) = left_side {
        let expression_id = current_left_side.expression_id();
        let leading_comments = ctx
            .comments()
            .comments_before(ctx.span(expression_id).start);
        let has_wrapping_leading_comment = leading_comments
            .iter()
            .copied()
            .any(|comment| comment.is_multiline_block() || comment.followed_by_newline());

        if has_wrapping_leading_comment {
            return true;
        }

        if !is_inside_yield && adjacent_statement_member_gap_has_comments(ctx, expression_id) {
            return true;
        }

        left_side = current_left_side.left(ctx);
    }

    false
}

/// Write one adjacent statement value inside explicit wrapping parentheses.
fn write_wrapped_adjacent_statement_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    content: &impl Format<DestackFormatContext<'ast>>,
) -> FormatResult<()> {
    write!(
        f,
        [
            space(),
            token("("),
            block_indent(content),
            hard_line_break(),
            token(")")
        ]
    )
}

/// Write one wrapped adjacent statement expression, preserving ternary expansion.
fn write_wrapped_adjacent_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let wrapped_expression_id = adjacent_statement_wrapped_expression(f.context(), expression_id);
    let wrapped_value = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if matches!(
            f.context().tree.get(wrapped_expression_id),
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        ) {
            return write_expanded_adjacent_statement_value(f, wrapped_expression_id);
        }

        write!(f, [wrapped_expression_id])
    });

    write_wrapped_adjacent_statement_value(f, &wrapped_value)
}

/// Return the expression that should print inside an adjacent wrapper.
fn adjacent_statement_wrapped_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = expression_id;

    loop {
        let Expression::Parenthesized { expression } = context.tree.get(current_id) else {
            return current_id;
        };

        current_id = *expression;
    }
}

/// Write one expanded adjacent statement value, preserving ternary expansion.
fn write_expanded_adjacent_statement_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if matches!(
        f.context().tree.get(expression_id),
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    ) {
        return format_expanded_ternary_expression(f, expression_id);
    }

    write!(
        f,
        [expand_parent(), group(&expression_id).should_expand(true)]
    )
}

/// Return one adjacent statement sequence expression after transparent wrappers.
fn adjacent_statement_sequence_value(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
    value_check_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match context.tree.get(value_check_id) {
        Expression::SequenceExpression { .. } => Some(value_check_id),
        _ => match context.tree.get(value_id) {
            Expression::SequenceExpression { .. } => Some(value_id),
            _ => None,
        },
    }
}

/// Format one sequence adjacent argument with explicit wrapping.
fn format_sequence_adjacent_statement_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    sequence_value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let prefix_annotation_source_id = if f.context().has_prefix_annotation(value_id) {
        Some(value_id)
    } else if f.context().has_prefix_annotation(sequence_value_id) {
        Some(sequence_value_id)
    } else {
        None
    };
    let grouped_sequence = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if let Some(prefix_annotation_source_id) = prefix_annotation_source_id {
            write!(
                f,
                [prefix_annotations(f.context(), prefix_annotation_source_id)]
            )?;
        }

        write!(f, [token("(")])?;
        write_expression_without_prefix_annotations(f, sequence_value_id)?;
        write!(f, [token(")")])
    });

    write!(
        f,
        [
            space(),
            token("("),
            block_indent(&grouped_sequence),
            hard_line_break(),
            token(")")
        ]
    )
}

/// Format one adjacent return, throw, or yield argument.
pub(crate) fn format_adjacent_statement_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let value_check_id = transparent_inner_expression(f.context(), value_id);
    let value_expression = f.context().tree.get(value_check_id);
    let sequence_value_id =
        adjacent_statement_sequence_value(f.context(), value_id, value_check_id);
    let value_has_leading_comments =
        adjacent_statement_argument_has_leading_comments(f.context(), value_id);

    if value_has_leading_comments {
        if let Some(sequence_value_id) = sequence_value_id {
            return format_sequence_adjacent_statement_argument(f, value_id, sequence_value_id);
        }

        return write_wrapped_adjacent_statement_expression(f, value_id);
    }

    let value_is_unwrapped_sequence =
        matches!(value_expression, Expression::SequenceExpression { .. });
    let should_wrap_value = value_is_unwrapped_sequence;

    if should_wrap_value {
        let wrapped_value =
            format_with(|f| write_expanded_adjacent_statement_value(f, value_check_id));
        return write_wrapped_adjacent_statement_value(f, &wrapped_value);
    }

    write!(f, [space(), value_id])?;
    Ok(())
}

/// Return whether expression has any prefix annotation.
fn expression_has_effective_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let annotations = context.annotation_ids(expression_id);
    if annotations.is_empty() {
        return false;
    }

    annotations.iter().copied().any(|annotation_id| {
        matches!(
            context.annotation(annotation_id).position,
            DecoratorPosition::LinePrefix | DecoratorPosition::BlockPrefix
        )
    })
}

/// Write one grouped `if (...) <body>` clause.
fn write_if_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    condition: &IfCondition,
    then_expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let empty_statement_body = match f.context().tree.get(then_expression_id) {
        Expression::Block(block_id) if is_empty_statement_block(f.context(), *block_id) => {
            Some(*block_id)
        }
        _ => None,
    };

    let head = format_with(|f| {
        match condition {
            IfCondition::Expression { condition } => {
                write_if_or_while_test_expression(f, *condition)?;
            }
            IfCondition::Let {
                kind,
                mutability: _,
                declarator,
            } => {
                match kind {
                    LetKind::Let => write!(f, [Keyword::Let])?,
                    LetKind::Var => write!(f, [Keyword::Var])?,
                    LetKind::Const => write!(f, [Keyword::Const])?,
                }

                write!(f, [space()])?;
                format_declarator(f, f.context().tree, *declarator)?;
            }
        }

        if let Some(empty_statement_body) = empty_statement_body {
            write_comments_for_empty_statement_body(f, empty_statement_body)?;
        }

        Ok(())
    });
    let body = format_with(|f| write_control_branch_after_head(f, then_expression_id));

    write!(
        f,
        [group(&format_args![
            Keyword::If,
            space(),
            token("("),
            group(&soft_block_indent(&head)),
            token(")"),
            body,
        ])]
    )
}

/// Write one control branch after its head.
pub(crate) fn write_control_branch_after_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    branch_expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // transparent statement wrappers
    if let Some(empty_block_id) = transparent_empty_control_body(f.context(), branch_expression_id)
    {
        format_empty_statement_body_after_head(f, empty_block_id)?;
        return Ok(());
    }

    // transparent statement wrappers
    if let Some(inner_expression_id) =
        transparent_control_body_expression(f.context(), branch_expression_id)
    {
        format_statement_body_expression_after_head(f, inner_expression_id)?;
        return Ok(());
    }

    let branch_expression = f.context().tree.get(branch_expression_id);
    match branch_expression {
        Expression::Block(block_id) => {
            write!(f, [prefix_annotations(f.context(), branch_expression_id)])?;
            format_statement_body_block_after_head(f, *block_id)?;
            write!(
                f,
                [infix_or_postfix_annotations(
                    f.context(),
                    branch_expression_id
                )]
            )?;
        }
        _ => {
            format_statement_body_expression_after_head(f, branch_expression_id)?;
            write!(f, [postfix_annotations(f.context(), branch_expression_id)])?;
        }
    }

    Ok(())
}

/// Write spacing and comments between one `then` branch and its `else`.
fn write_if_else_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    if_expression_id: LocalNodeId<Expression>,
    then_expression_id: LocalNodeId<Expression>,
    else_expression_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let else_has_effective_prefix_annotation =
        expression_has_effective_prefix_annotation(f.context(), else_expression_id);

    if else_has_effective_prefix_annotation || f.context().has_postfix_annotation(if_expression_id)
    {
        write!(f, [postfix_annotations(f.context(), if_expression_id)])?;
    }

    let else_clause_span = f
        .context()
        .tree
        .get_side_span(
            if_expression_id,
            NodeSpanType::Region(NodeSpanRegion::Clause),
        )
        .expect("if expressions with else branches must record the else clause span");
    let else_start = else_clause_span.start;
    let comments = f.context().comments().comments_before(else_start).to_vec();
    let has_line_comment = comments.iter().any(|comment| comment.is_line());
    let then_end = f.context().span(then_expression_id).end;
    let has_boundary_comment_before_else = f
        .context()
        .comments()
        .printed_comments()
        .last()
        .is_some_and(|comment| comment.span.start >= then_end && comment.span.end <= else_start);
    let has_dangling_comments = !comments.is_empty() || has_boundary_comment_before_else;
    let then_is_explicit_block = matches!(
        f.context().tree.get(then_expression_id),
        Expression::Block(block_id)
            if f.context().tree.get(*block_id).format == BlockFormat::Explicit
    );
    let else_on_same_line = then_is_explicit_block && (!has_line_comment || !has_dangling_comments);
    let line_comment_after_explicit_block =
        then_is_explicit_block && has_line_comment && has_dangling_comments;

    if line_comment_after_explicit_block {
        write!(
            f,
            [
                FormatTrailingComments::Comments(&comments),
                hard_line_break()
            ]
        )?;
        return Ok(else_has_effective_prefix_annotation);
    } else if else_on_same_line {
        write!(
            f,
            [
                space(),
                has_dangling_comments.then_some(line_suffix_boundary())
            ]
        )?;
    } else {
        write!(f, [hard_line_break()])?;
    }

    if has_dangling_comments && let Some(first_comment) = comments.first() {
        if f.context()
            .source_text()
            .get_lines_before(first_comment.span, f.context().comments())
            > 1
        {
            write!(f, [empty_line()])?;
        }

        write!(
            f,
            [FormatDanglingComments::Comments {
                comments: &comments,
                indent: DanglingIndentMode::None
            }]
        )?;

        if has_line_comment {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(else_has_effective_prefix_annotation)
}

/// Format one `else` branch and return the next chained `if`, if any.
fn format_if_else_alternate<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    if_expression_id: LocalNodeId<Expression>,
    then_expression_id: LocalNodeId<Expression>,
    else_expression_id: LocalNodeId<Expression>,
) -> FormatResult<Option<LocalNodeId<Expression>>> {
    let else_has_effective_prefix_annotation =
        write_if_else_separator(f, if_expression_id, then_expression_id, else_expression_id)?;

    match f.context().tree.get(else_expression_id) {
        Expression::If { .. } => {
            write!(f, [prefix_annotations(f.context(), else_expression_id)])?;
            write!(f, [Keyword::Else, space()])?;
            Ok(Some(else_expression_id))
        }
        Expression::Block(else_block_id)
            if transparent_empty_control_body(f.context(), else_expression_id).is_none()
                && transparent_control_body_expression(f.context(), else_expression_id)
                    .is_none() =>
        {
            write!(f, [prefix_annotations(f.context(), else_expression_id)])?;
            write!(f, [Keyword::Else, space()])?;
            format_block(f, *else_block_id)?;

            if f.context().has_postfix_annotation(else_expression_id) {
                write!(f, [postfix_annotations(f.context(), else_expression_id)])?;
            }

            Ok(None)
        }
        Expression::Block(_) => {
            write!(f, [Keyword::Else])?;

            if let Some(empty_block_id) =
                transparent_empty_control_body(f.context(), else_expression_id)
            {
                if !else_has_effective_prefix_annotation {
                    write!(f, [space()])?;
                }
                format_empty_statement_body_after_head(f, empty_block_id)?;
            } else if let Some(inner_expression_id) =
                transparent_control_body_expression(f.context(), else_expression_id)
            {
                let body = format_with(|f| {
                    format_statement_body_expression_after_head(f, inner_expression_id)
                });
                write!(f, [group(&body)])?;
            }

            Ok(None)
        }
        _ => {
            write!(f, [Keyword::Else])?;

            let body =
                format_with(|f| format_statement_body_expression_after_head(f, else_expression_id));
            write!(f, [group(&body)])?;
            Ok(None)
        }
    }
}

/// Walk a chain of if expressions and collect the if/else if/else nodes.
pub(crate) fn format_if_else_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // walk the chain
    let mut next_if_id = node_id;
    loop {
        let if_node = f.context().tree.get(next_if_id);
        match if_node {
            // if or else if
            Expression::If {
                kind: _, // we turn everything into regular ifs
                condition,
                then_expression: then_expression_id,
                else_expression: else_expression_id,
            } => {
                write_if_clause(f, condition, *then_expression_id)?;

                // next node
                if let Some(else_expression) = else_expression_id {
                    match format_if_else_alternate(
                        f,
                        next_if_id,
                        *then_expression_id,
                        *else_expression,
                    )? {
                        Some(next_else_if_id) => {
                            next_if_id = next_else_if_id;
                        }
                        None => {
                            break;
                        }
                    }
                } else {
                    // bare if
                    write!(f, [postfix_annotations(f.context(), next_if_id)])?;
                    break;
                }
            }
            // shouldn't be anything else
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unexpected expression kind for if chain",
                });
            }
        }
    }
    Ok(())
}

/// Format a match selector according to the selected case style.
fn format_selector_with_style(
    f: &mut DestackFormatter<'_, '_>,
    selector: &MatchSelector,
    is_switch_style: bool,
) -> FormatResult<()> {
    if !is_switch_style {
        match selector {
            MatchSelector::Pattern { pattern, guard } => {
                write!(f, [*pattern])?;
                if let Some(guard) = guard {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::If,
                            space(),
                            token("("),
                            *guard,
                            token(")")
                        ]
                    )?;
                }
            }
            MatchSelector::Default => {
                write!(f, [token("_")])?;
            }
        }
    } else {
        match selector {
            MatchSelector::Pattern { pattern, guard } => {
                write!(f, [Keyword::Case, space(), *pattern, token(":")])?;
                if let Some(guard) = guard {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::If,
                            space(),
                            token("("),
                            *guard,
                            token(")")
                        ]
                    )?;
                }
            }
            MatchSelector::Default => {
                write!(f, [Keyword::Default, token(":")])?;
            }
        }
    }

    Ok(())
}

/// Format one return expression in statement position.
pub(crate) fn format_return_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    value: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // return keyword
    write!(f, [token("return")])?;

    // return value
    if let Some(value_id) = value {
        let value_expression = tree.get(value_id);

        // tree returns may need wrapping parens to keep multiline layout stable
        if let Expression::TreeExpression {
            arguments,
            elements,
            ..
        } = value_expression
        {
            let has_children = elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty());
            let has_multiple_attributes = arguments
                .as_ref()
                .is_some_and(|arguments| arguments.len() > 1);
            let should_wrap_tree_return = has_children
                || has_multiple_attributes
                || tree_literal_should_break(f.context(), arguments, elements);

            if should_wrap_tree_return {
                write!(
                    f,
                    [
                        space(),
                        token("("),
                        block_indent(&value_id),
                        hard_line_break(),
                        token(")")
                    ]
                )?;
            } else {
                format_adjacent_statement_argument(f, value_id)?;
            }
        } else {
            format_adjacent_statement_argument(f, value_id)?;
        }
    }

    Ok(())
}

/// Format one yield expression in statement position.
pub(crate) fn format_yield_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    cardinality: YieldCardinality,
    value: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // yield keyword
    write!(f, [Keyword::Yield])?;
    if cardinality == YieldCardinality::Generator {
        write!(f, [token("*")])?;
    }

    // yield value
    if let Some(value_id) = value {
        format_adjacent_statement_argument(f, value_id)?;
    }

    Ok(())
}

/// Format one throw expression in statement position.
pub(crate) fn format_throw_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [token("throw")])?;
    format_adjacent_statement_argument(f, value_id)?;

    Ok(())
}

/// Format one break expression in statement position.
pub(crate) fn format_break_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    label: &Option<StringId>,
    value: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    write!(f, [Keyword::Break])?;

    if let Some(label) = label {
        if f.context().options.language_type.is_destack() {
            write!(f, [space(), token(":"), label])?;
        } else {
            write!(f, [space(), label])?;
        }
    }

    if let Some(value) = value {
        write!(f, [space(), value])?;
    }

    Ok(())
}

/// Format one continue expression in statement position.
pub(crate) fn format_continue_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    label: &Option<StringId>,
) -> FormatResult<()> {
    write!(f, [Keyword::Continue])?;

    if let Some(label) = label {
        if f.context().options.language_type.is_destack() {
            write!(f, [space(), token(":"), label])?;
        } else {
            write!(f, [space(), label])?;
        }
    }

    Ok(())
}

/// Return whether a control-flow statement body should be preceded by a space.
fn statement_body_requires_head_space(
    context: &DestackFormatContext<'_>,
    body: LocalNodeId<Block>,
) -> bool {
    if is_empty_statement_block(context, body) {
        return context
            .comments()
            .has_comment_before(context.span(body).start);
    }

    expression_body_requires_head_space(context, LocalNodeId::<Expression>::new(body.id))
}

/// Format a `while` or `do while` expression.
pub(crate) fn format_while_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    kind: WhileKind,
    condition: LocalNodeId<Expression>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    match kind {
        // while (<condition>) <body>
        WhileKind::While => {
            let head = format_with(|f| {
                write_if_or_while_test_expression(f, condition)?;
                write_comments_for_empty_statement_body(f, body)
            });

            write!(f, [Keyword::While, space(), token("(")])?;
            write_grouped_control_head(f, &head)?;
            write!(f, [token(")")])?;
            if statement_body_requires_head_space(f.context(), body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, body)?;
        }
        // do <body> while (<condition>)
        WhileKind::DoWhile => {
            let is_block_body = f.context().tree.get(body).format == BlockFormat::Explicit;

            write!(f, [Keyword::Do])?;
            if statement_body_requires_head_space(f.context(), body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, body)?;

            if is_block_body {
                write!(f, [space()])?;
            } else {
                write!(f, [hard_line_break()])?;
            }

            write!(
                f,
                [
                    Keyword::While,
                    space(),
                    token("("),
                    format_with(|f| write_grouped_control_head(
                        f,
                        &format_with(|f| {
                            write_if_or_while_test_expression(f, condition)?;
                            write_comments_for_empty_statement_body(f, body)
                        }),
                    )),
                    token(")"),
                ]
            )?;
        }
    }

    Ok(())
}

/// Format a `for each` expression.
pub(crate) fn format_for_each_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    asynchrony: Asynchrony,
    kind: ForEachKind,
    binding: &ForEachBinding,
    iterator: LocalNodeId<Expression>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // for header
    write!(f, [Keyword::For, space()])?;
    if asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Await, space()])?;
    }
    let keyword = match kind {
        ForEachKind::In => Keyword::In,
        ForEachKind::Of => Keyword::Of,
    };

    // binding
    write!(f, [token("(")])?;
    match binding {
        ForEachBinding::Pattern {
            pattern,
            declaration_kind,
        } => {
            // explicit declaration kind
            if let Some(declaration_kind) = declaration_kind {
                let keyword = match declaration_kind {
                    ForEachDeclarationKind::Var => Keyword::Var,
                    ForEachDeclarationKind::Let => Keyword::Let,
                    ForEachDeclarationKind::Const => Keyword::Const,
                };
                write!(f, [keyword, space()])?;
                format_for_each_binding_pattern(f, *pattern)?;
            }
            // source keyword recovery
            else {
                let source_keyword =
                    detect_for_each_binding_keyword(f.context(), node_id, *pattern);
                if let Some(keyword) = source_keyword {
                    write!(f, [keyword, space()])?;
                    format_for_each_binding_pattern(f, *pattern)?;
                } else {
                    let pattern_node = tree.get(*pattern);
                    let should_prefix_const = matches!(
                        pattern_node,
                        Pattern::Binding {
                            mutability: Some(Mutability::Immutable),
                            pattern: None,
                            ..
                        }
                    );

                    // keep explicit const for simple immutable bindings
                    if should_prefix_const {
                        write!(f, [Keyword::Const, space()])?;
                    }
                    write!(f, [pattern])?;
                }
            }
        }
        ForEachBinding::Using {
            asynchrony,
            pattern,
        } => {
            if *asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Await, space()])?;
            }
            write!(f, [Keyword::Using, space(), pattern])?;
        }
    }

    // iterator + body
    write!(
        f,
        [
            space(),
            keyword,
            space(),
            iterator,
            format_with(|f| write_comments_for_empty_statement_body(f, body)),
            token(")")
        ]
    )?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)?;

    Ok(())
}

/// Format a classic `for` expression.
pub(crate) fn format_for_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    initialization: Option<LocalNodeId<Expression>>,
    condition: Option<LocalNodeId<Expression>>,
    increment: Option<LocalNodeId<Expression>>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(
        f,
        [
            Keyword::For,
            space(),
            token("("),
            initialization,
            token(";"),
            space(),
            condition,
            token(";"),
            space(),
            increment,
            format_with(|f| write_comments_for_empty_statement_body(f, body)),
            token(")")
        ]
    )?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)
}

/// Format a `loop` expression.
pub(crate) fn format_loop_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(f, [Keyword::Loop])?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)
}

/// Write one catch parameter inside parentheses.
fn write_catch_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    catch_pattern: LocalNodeId<Pattern>,
    catch_ty: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    let parameter_end = catch_ty
        .map(|catch_ty| f.context().span(catch_ty).end)
        .unwrap_or_else(|| f.context().span(catch_pattern).end);
    let trailing_comments = f
        .context()
        .comments()
        .comments_before_character(parameter_end, b')')
        .to_vec();
    let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [catch_pattern])?;

        if let Some(catch_ty) = catch_ty {
            write!(f, [token(":"), space(), catch_ty])?;
        }

        write!(f, [FormatTrailingComments::Comments(&trailing_comments)])
    });

    write!(
        f,
        [token("("), group(&soft_block_indent(&content)), token(")")]
    )
}

/// Format a `try` expression.
pub(crate) fn format_try_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    try_expression: LocalNodeId<Expression>,
    catch_pattern: Option<LocalNodeId<Pattern>>,
    catch_ty: Option<LocalNodeId<TypeExpression>>,
    catch_expression: Option<LocalNodeId<Expression>>,
    finally_expression: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // try block
    write!(f, [Keyword::Try])?;
    if let Expression::Block(block_id) = f.context().tree.get(try_expression) {
        if statement_body_requires_head_space(f.context(), *block_id) {
            write!(f, [space()])?;
        }
    } else {
        write!(f, [space()])?;
    }
    write!(f, [try_expression])?;

    // catch block
    if let Some(catch_expression) = catch_expression {
        write!(f, [space(), Keyword::Catch])?;
        if let Some(catch_pattern) = catch_pattern {
            write!(f, [space()])?;
            write_catch_parameter(f, catch_pattern, catch_ty)?;
        }
        if let Expression::Block(block_id) = f.context().tree.get(catch_expression) {
            if statement_body_requires_head_space(f.context(), *block_id) {
                write!(f, [space()])?;
            }
        } else {
            write!(f, [space()])?;
        }
        write!(f, [catch_expression])?;
    }

    // finally block
    if let Some(finally_expression) = finally_expression {
        write!(f, [space(), Keyword::Finally])?;
        if let Expression::Block(block_id) = f.context().tree.get(finally_expression) {
            if statement_body_requires_head_space(f.context(), *block_id) {
                write!(f, [space()])?;
            }
        } else {
            write!(f, [space()])?;
        }
        write!(f, [finally_expression])?;
    }

    Ok(())
}

/// Format one match case with the selected style.
pub(crate) fn format_match_case_with_style<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    case_id: LocalNodeId<MatchCase>,
    is_switch_style: bool,
) -> FormatResult<()> {
    let case = f.context().tree.get(case_id);

    // case prefix
    write_match_case_prefix(f, case_id)?;

    // selector, separator, and body
    match case {
        MatchCase::Expression { selector, body } => {
            format_selector_with_style(f, selector, is_switch_style)?;

            let format_switch_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                format_statement_body_expression(f, *body)
            });

            if !is_switch_style {
                write!(f, [space(), token("=>"), space(), *body])?;
            } else if switch_case_expression_body_is_explicit_block(f.context(), *body) {
                write!(f, [space(), *body])?;
            } else {
                write!(f, [hard_line_break(), block_indent(&format_switch_body)])?;
            }
        }
        MatchCase::Block { selector, body } => {
            format_selector_with_style(f, selector, is_switch_style)?;
            if !is_switch_style {
                write!(f, [space(), token("=>"), space(), *body])?;
            } else {
                let block = f.context().tree.get(*body);
                if block.format == BlockFormat::Explicit {
                    write!(f, [space(), *body])?;
                } else if !block.is_empty() {
                    write!(
                        f,
                        [
                            hard_line_break(),
                            block_indent(&block_statement_sequence(*body, false, None))
                        ]
                    )?;
                }
            }
        }
    }

    // case postfix
    write!(f, [infix_or_postfix_annotations(f.context(), case_id)])?;

    Ok(())
}

/// Return whether one switch case expression body is one explicit block expression.
fn switch_case_expression_body_is_explicit_block(
    context: &DestackFormatContext<'_>,
    body_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Block(block_id) = context.tree.get(body_expression_id) else {
        return false;
    };

    let block = context.tree.get(*block_id);
    block.format == BlockFormat::Explicit
}

impl<'ast> FormatNode<'ast, MatchCase> for MatchCase {
    fn format_node(
        &self,
        node_id: LocalNodeId<MatchCase>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_match_case_with_style(f, node_id, false)
    }
}

/// Format a match expression.
pub(crate) fn format_match<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    include_prefix: bool,
) -> FormatResult<()> {
    let match_node = f.context().tree.get(node_id);
    let Expression::Match { kind, value, cases } = &match_node else {
        return Err(FormatError::SyntaxError {
            message: "invalid match expression",
        });
    };
    let kind = *kind;
    let is_switch_style = matches!(kind, MatchKind::Switch);

    if include_prefix {
        // match/switch <expression>
        let keyword = match kind {
            MatchKind::Match => Keyword::Match,
            MatchKind::Switch => Keyword::Switch,
        };
        write!(f, [keyword, space()])?;
    }

    write!(f, [token("("), value, token(")")])?;

    // empty match body
    if cases.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        write!(f, [postfix_annotations(f.context(), node_id)])?;
        return Ok(());
    }

    // match/switch cases
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            let mut first = true;
            for case_id in cases {
                if !first {
                    write!(f, [hard_line_break()])?;
                }
                first = false;
                format_match_case_with_style(f, *case_id, is_switch_style)?;
            }
            Ok(())
        })),])]
    )?;
    write!(f, [block_infix_annotations(f.context(), node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;

    Ok(())
}
