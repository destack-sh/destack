use super::declarator::format_declarator;
use super::dispatch::format_expression;
use super::{format_expanded_ternary_expression, write_expression_without_trailing_comments};
use crate::annotation::{
    DanglingIndentMode, FormatDanglingComments, FormatLeadingComments, FormatTrailingComments,
    block_infix_annotations, format_leading_comments, infix_or_postfix_annotations,
    postfix_annotations, prefix_annotations, prefix_comment_nodes, write_annotation_sequence,
    write_comment_slice,
};
use crate::declaration::sequence::block_statement_sequence;
use crate::declaration::statement::{format_block, format_block_wide};
use crate::declaration::{
    empty_block_with_infix_annotations, statement_wrapper_needs_semicolon,
    write_statement_terminator, write_statement_terminator_after_anchor,
};
use crate::expression::ExpressionLeftPath;
use crate::file::node_has_ignore_directive;
use crate::tree::tree_literal_should_break;
use crate::{FormatNode, TsppFormatContext, TsppFormatter};
use tspp_core::{StringId, ensure_sufficient_stack};
use tspp_dir::{
    Asynchrony, BindingKeyword, Block, BlockForm, Catch, Condition, ConditionOperand,
    DecoratorPosition, Expression, ForEachBinding, IfForm, Keyword, LetKind, LocalNodeId, MatchArm,
    Node, NodeType, Pattern, SwitchCase, SwitchSelector, Tree, TreeStore, TypeExpression,
    WhileForm, YieldCardinality,
};
use tspp_fir::format::{Format, FormatError, FormatResult};
use tspp_fir::prelude::{
    block_indent, empty_line, expand_parent, format_with, group, hard_line_break,
    line_suffix_boundary, soft_block_indent, soft_line_break_or_space, soft_line_indent_or_space,
    space, token,
};
use tspp_fir::{best_fitting, format_args, write};
use tspp_source::{NodeSpanRegion, NodeSpanType, Span};

/// Write one condition expression before the closing `)`.
fn write_condition_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
    head: &impl Format<'ast, TsppFormatContext<'ast>>,
) -> FormatResult<()> {
    write!(f, [group(&soft_block_indent(head))])
}

/// Write comments that belong to one empty statement body before its semicolon.
fn write_comments_for_empty_statement_body<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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

/// Format one control-flow body expression with statement-separator semantics.
fn format_statement_body_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);
    let is_ignored = node_has_ignore_directive(f.context(), expression_id);
    let expression_span = f.context().span(expression_id);
    let token_start = f.context().expression_token_start(expression_id);
    let token_start_span = Span::new(expression_span.file, token_start, token_start);

    write!(f, [format_leading_comments(token_start_span)])?;

    write!(f, [prefix_annotations(f.context(), expression_id)])?;
    format_expression(f, expression_id, expression, is_ignored)?;

    if statement_wrapper_needs_semicolon(f.context(), expression_id) {
        write_statement_terminator(f, expression_id)?;
    }

    let if_chain_handles_annotations = matches!(
        expression,
        Expression::If {
            form: IfForm::If,
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

/// Write leading comments and prefix annotations for one match arm or switch case.
fn write_case_prefix<'ast, T>(
    f: &mut TsppFormatter<'ast, '_>,
    case_id: LocalNodeId<T>,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
{
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

/// Write one match arm guard.
fn write_match_guard<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    arm_id: LocalNodeId<MatchArm>,
    pattern_id: LocalNodeId<Pattern>,
    guard: &Condition,
) -> FormatResult<()> {
    let guard_clause_span = f
        .context()
        .tree
        .get_side_span(arm_id, NodeSpanType::Region(NodeSpanRegion::Guard))
        .ok_or(FormatError::SyntaxError {
            message: "match guard requires a clause span",
        })?;
    let guard_prefix_comments = f
        .context()
        .comments()
        .comments_in_range(f.context().span(pattern_id).end, guard_clause_span.start)
        .to_vec();
    let guard = format_with(|f| write_condition(f, guard));

    write!(
        f,
        [
            FormatTrailingComments::Comments(&guard_prefix_comments),
            space(),
            Keyword::If,
            space(),
            token("("),
            format_with(|f| write_grouped_control_head(f, &guard)),
            token(")")
        ]
    )
}

/// Format a statement body block, preserving wrapper semantics.
pub(crate) fn format_statement_body_block<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
        let expression_id = block.first_expression().ok_or(FormatError::SyntaxError {
            message: "nonempty statement body requires one expression",
        })?;
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
    f: &mut TsppFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
    force_expanded_body: bool,
) -> FormatResult<()> {
    ensure_sufficient_stack(|| {
        format_statement_body_block_after_head_inner(f, block_id, force_expanded_body)
    })
}

/// Format one block-backed statement body.
fn format_statement_body_block_after_head_inner<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
    force_expanded_body: bool,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    if !is_statement_wrapper_block(f.context(), block_id) {
        write!(f, [space()])?;

        if !force_expanded_body {
            return format_block(f, block_id);
        }

        return format_block_wide(f, block_id);
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
        let expression_id = block.first_expression().ok_or(FormatError::SyntaxError {
            message: "nonempty control body requires one expression",
        })?;
        let has_leading_comments = control_body_has_leading_comments(f.context(), expression_id);
        let body = format_with(|f| format_statement_body_expression(f, expression_id));

        if expression_has_block_prefix_annotation(f.context(), expression_id)
            || has_leading_comments
            || force_expanded_body
        {
            return write!(f, [hard_line_break(), group(&block_indent(&body))]);
        }

        return write!(f, [soft_line_indent_or_space(&body)]);
    }

    write!(f, [space(), block_id])
}

/// Return whether comments occur between a control head and its body.
fn control_body_has_leading_comments(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_span = context.span(expression_id);
    let head_end = context
        .previous_token_before_span(expression_span)
        .map_or(expression_span.start, |token| token.span.end);

    context
        .comments()
        .has_comment_in_range(head_end, expression_span.start)
}

/// Return true when this block originated from a statement wrapper instead of braces.
fn is_statement_wrapper_block<'ast>(
    context: &TsppFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    block.form == BlockForm::Implicit
}

/// Return true when this block is an empty statement wrapper.
pub(crate) fn is_empty_statement_block<'ast>(
    context: &TsppFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    is_statement_wrapper_block(context, block_id) && block.is_empty()
}

/// Format a for each binding pattern without repeating root mutability keywords.
pub(crate) fn format_for_each_binding_pattern<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let Expression::Block(block_id) = context.tree.get(expression_id) else {
        return None;
    };

    let block = context.tree.get(*block_id);
    if block.form != BlockForm::Implicit || block.len() != 1 {
        return None;
    }

    if context.has_annotation(expression_id) || context.has_annotation(*block_id) {
        return None;
    }

    block.first_expression()
}

/// Return the empty implicit statement wrapper behind one transparent control body.
fn transparent_empty_control_body(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Block>> {
    let Expression::Block(block_id) = context.tree.get(expression_id) else {
        return None;
    };

    let block = context.tree.get(*block_id);
    if block.form != BlockForm::Implicit || !block.is_empty() {
        return None;
    }

    if context.has_annotation(expression_id) || context.has_annotation(*block_id) {
        return None;
    }

    Some(*block_id)
}

/// Format one transparent empty statement body after a control-flow head.
fn format_empty_statement_body_after_head<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    write_statement_terminator_after_anchor(f, f.context().span(block_id).start)
}

/// Format one non-block statement body after a control-flow head.
fn format_statement_body_expression_after_head<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    force_expanded_body: bool,
) -> FormatResult<()> {
    let has_leading_comments = control_body_has_leading_comments(f.context(), expression_id);
    let body = format_with(|f| format_statement_body_expression(f, expression_id));

    if expression_has_block_prefix_annotation(f.context(), expression_id)
        || has_leading_comments
        || force_expanded_body
    {
        write!(f, [hard_line_break(), group(&block_indent(&body))])?;
        return Ok(());
    }

    let flat_body = format_with(|f| write!(f, [space(), &body]));
    let expanded_body = format_with(|f| {
        write!(
            f,
            [group(&soft_line_indent_or_space(&body)).should_expand(true)]
        )
    });

    write!(f, [best_fitting![flat_body, expanded_body]])
}

/// Return whether one adjacent argument is nested directly inside `yield`.
fn adjacent_statement_argument_is_inside_yield(
    ctx: &TsppFormatContext<'_>,
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
    ctx: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let (left, property_start) = match ctx.tree.get(expression_id) {
        Expression::Member { left, .. } => {
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
    ctx: &TsppFormatContext<'_>,
    argument_id: LocalNodeId<Expression>,
) -> bool {
    let is_inside_yield = adjacent_statement_argument_is_inside_yield(ctx, argument_id);
    let mut left_path = Some(ExpressionLeftPath::new(argument_id));

    while let Some(current) = left_path {
        let expression_id = current.expression_id();
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

        left_path = current.next(ctx);
    }

    false
}

/// Write one adjacent statement value inside explicit wrapping parentheses.
fn write_wrapped_adjacent_statement_value<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    content: &impl Format<'ast, TsppFormatContext<'ast>>,
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
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let leading_expression_id = ExpressionLeftPath::new(expression_id)
        .leftmost(f.context())
        .expression_id();
    let leading_expression_span = f.context().span(leading_expression_id);
    let leading_token_start = f.context().expression_token_start(leading_expression_id);
    let leading_token_span = Span::new(
        leading_expression_span.file,
        leading_token_start,
        leading_token_start,
    );
    let wrapped_value = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        write!(f, [format_leading_comments(leading_token_span)])?;

        if matches!(
            f.context().tree.get(expression_id),
            Expression::If {
                form: IfForm::Ternary,
                ..
            }
        ) {
            return write_expanded_adjacent_statement_value(f, expression_id);
        }

        write!(f, [expression_id])
    });

    write_wrapped_adjacent_statement_value(f, &wrapped_value)
}

/// Write one expanded adjacent statement value, preserving ternary expansion.
fn write_expanded_adjacent_statement_value<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if matches!(
        f.context().tree.get(expression_id),
        Expression::If {
            form: IfForm::Ternary,
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

/// Format one adjacent return or yield argument.
pub(crate) fn format_adjacent_statement_argument<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let value_has_leading_comments =
        adjacent_statement_argument_has_leading_comments(f.context(), value_id);

    if value_has_leading_comments {
        return write_wrapped_adjacent_statement_expression(f, value_id);
    }

    write!(f, [space(), value_id])?;
    Ok(())
}

/// Return whether expression has any prefix annotation.
fn expression_has_effective_prefix_annotation(
    context: &TsppFormatContext<'_>,
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
    f: &mut TsppFormatter<'ast, '_>,
    condition: &Condition,
    then_expression_id: LocalNodeId<Expression>,
    expand_branch_bodies: bool,
) -> FormatResult<()> {
    let empty_statement_body = match f.context().tree.get(then_expression_id) {
        Expression::Block(block_id) if is_empty_statement_block(f.context(), *block_id) => {
            Some(*block_id)
        }
        _ => None,
    };

    let head = format_with(|f| {
        write_condition(f, condition)?;

        if let Some(empty_statement_body) = empty_statement_body {
            write_comments_for_empty_statement_body(f, empty_statement_body)?;
        }

        Ok(())
    });
    let body = format_with(|f| {
        write_control_branch_after_head_expanding_body(f, then_expression_id, expand_branch_bodies)
    });

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

/// Write one condition.
fn write_condition<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    condition: &Condition,
) -> FormatResult<()> {
    for (index, operand) in condition.operands.iter().enumerate() {
        if index > 0 {
            write!(f, [space(), token("&&"), space()])?;
        }

        write_condition_operand(f, operand)?;
    }

    Ok(())
}

/// Write one condition operand.
fn write_condition_operand<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    operand: &ConditionOperand,
) -> FormatResult<()> {
    match operand {
        // boolean condition
        ConditionOperand::Expression { condition } => {
            write_condition_expression(f, *condition)?;
        }
        // pattern binding condition
        ConditionOperand::Binding {
            kind,
            mutability: _,
            declarator,
        } => {
            match kind {
                LetKind::Let => write!(f, [Keyword::Let])?,
                LetKind::Const => write!(f, [Keyword::Const])?,
            }

            write!(f, [space()])?;
            format_declarator(f, f.context().tree, *declarator)?;
        }
    }

    Ok(())
}

/// Write one control branch after its head.
pub(crate) fn write_control_branch_after_head<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    branch_expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_control_branch_after_head_expanding_body(f, branch_expression_id, false)
}

/// Write one control branch after its head.
fn write_control_branch_after_head_expanding_body<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    branch_expression_id: LocalNodeId<Expression>,
    force_expanded_body: bool,
) -> FormatResult<()> {
    // transparent statement wrappers
    if let Some(empty_block_id) = transparent_empty_control_body(f.context(), branch_expression_id)
    {
        format_empty_statement_body_after_head(f, empty_block_id)?;
        return Ok(());
    }

    // single-expression statement wrappers
    if let Some(inner_expression_id) =
        transparent_control_body_expression(f.context(), branch_expression_id)
    {
        let force_expanded_body = force_expanded_body
            && matches!(
                f.context().tree.get(inner_expression_id),
                Expression::TreeExpression { .. }
            );
        format_statement_body_expression_after_head(f, inner_expression_id, force_expanded_body)?;
        return Ok(());
    }

    let branch_expression = f.context().tree.get(branch_expression_id);
    match branch_expression {
        Expression::Block(block_id) => {
            write!(f, [prefix_annotations(f.context(), branch_expression_id)])?;
            format_statement_body_block_after_head(f, *block_id, force_expanded_body)?;
            write!(
                f,
                [infix_or_postfix_annotations(
                    f.context(),
                    branch_expression_id
                )]
            )?;
        }
        _ => {
            format_statement_body_expression_after_head(
                f,
                branch_expression_id,
                force_expanded_body,
            )?;
            write!(f, [postfix_annotations(f.context(), branch_expression_id)])?;
        }
    }

    Ok(())
}

/// Write spacing and comments between one `then` branch and its `else`.
fn write_if_else_separator<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
        .get_side_span(if_expression_id, NodeSpanType::Region(NodeSpanRegion::Else))
        .ok_or(FormatError::SyntaxError {
            message: "if expression with else branch requires an else span",
        })?;
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
            if f.context().tree.get(*block_id).is_explicit()
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
    f: &mut TsppFormatter<'ast, '_>,
    if_expression_id: LocalNodeId<Expression>,
    then_expression_id: LocalNodeId<Expression>,
    else_expression_id: LocalNodeId<Expression>,
    expand_branch_bodies: bool,
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

            if expand_branch_bodies {
                format_block_wide(f, *else_block_id)?;
            } else {
                format_block(f, *else_block_id)?;
            }

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
            } else {
                write_control_branch_after_head_expanding_body(
                    f,
                    else_expression_id,
                    expand_branch_bodies,
                )?;
            }

            Ok(None)
        }
        _ => {
            write!(f, [Keyword::Else])?;
            write_control_branch_after_head_expanding_body(
                f,
                else_expression_id,
                expand_branch_bodies,
            )?;
            Ok(None)
        }
    }
}

/// Walk a chain of if expressions and collect the if/else if/else nodes.
pub(crate) fn format_if_else_chain<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expand_branch_bodies: bool,
) -> FormatResult<()> {
    // walk the chain
    let mut next_if_id = node_id;
    loop {
        let if_node = f.context().tree.get(next_if_id);
        match if_node {
            // if or else if
            Expression::If {
                form: _, // we turn everything into regular ifs
                condition,
                then_expression: then_expression_id,
                else_expression: else_expression_id,
            } => {
                write_if_clause(f, condition, *then_expression_id, expand_branch_bodies)?;

                // next node
                if let Some(else_expression) = else_expression_id {
                    match format_if_else_alternate(
                        f,
                        next_if_id,
                        *then_expression_id,
                        *else_expression,
                        expand_branch_bodies,
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

/// Format one return expression in statement position.
pub(crate) fn format_return_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
            attributes,
            children,
            ..
        } = value_expression
        {
            let has_children = children
                .as_ref()
                .is_some_and(|children| !children.is_empty());
            let has_multiple_attributes = attributes
                .as_ref()
                .is_some_and(|attributes| attributes.len() > 1);
            let should_wrap_tree_return = has_children
                || has_multiple_attributes
                || tree_literal_should_break(f.context(), attributes, children);

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
    f: &mut TsppFormatter<'ast, '_>,
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

/// Format one break expression in statement position.
pub(crate) fn format_break_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    label: &Option<StringId>,
    value: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    write!(f, [Keyword::Break])?;

    if let Some(label) = label {
        write!(f, [space(), label])?;
    }

    if let Some(value) = value {
        // labeled values separate with a colon
        if label.is_some() {
            write!(f, [token(":")])?;
            write!(f, [space(), value])?;
        }
        // a lone identifier value keeps exactly one parenthesis pair, since a
        // bare identifier operand reads as a label
        else if let Some(identifier_id) = break_value_identifier(f.context(), *value) {
            write!(f, [space(), token("("), identifier_id, token(")")])?;
        }
        // other values print bare
        else {
            write!(f, [space(), value])?;
        }
    }

    Ok(())
}

/// Return the lone identifier inside one break value.
fn break_value_identifier(
    context: &TsppFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    matches!(context.tree.get(value_id), Expression::Identifier { .. }).then_some(value_id)
}

/// Format one continue expression in statement position.
pub(crate) fn format_continue_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    label: &Option<StringId>,
) -> FormatResult<()> {
    write!(f, [Keyword::Continue])?;

    if let Some(label) = label {
        write!(f, [space(), label])?;
    }

    Ok(())
}

/// Return whether a control-flow statement body should be preceded by a space.
fn statement_body_requires_head_space(
    context: &TsppFormatContext<'_>,
    body: LocalNodeId<Block>,
) -> bool {
    if is_empty_statement_block(context, body) {
        return context
            .comments()
            .has_comment_before(context.span(body).start);
    }

    true
}

/// Format a `while` or `do while` expression.
pub(crate) fn format_while_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    form: WhileForm,
    condition: &Condition,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    match form {
        // while (<condition>) <body>
        WhileForm::While => {
            let head = format_with(|f| {
                write_condition(f, condition)?;
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
        WhileForm::DoWhile => {
            let is_block_body = f.context().tree.get(body).is_explicit();

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
                            write_condition(f, condition)?;
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
    f: &mut TsppFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    asynchrony: Asynchrony,
    binding: &ForEachBinding,
    iterator: LocalNodeId<Expression>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    // for header
    write!(f, [Keyword::For, space()])?;
    if asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Await, space()])?;
    }

    // binding
    write!(f, [token("(")])?;
    match binding {
        ForEachBinding::Pattern { pattern, keyword } => {
            // explicit declaration kind
            if let Some(keyword) = keyword {
                let keyword = match keyword {
                    BindingKeyword::Let => Keyword::Let,
                    BindingKeyword::Const => Keyword::Const,
                };
                write!(f, [keyword, space()])?;
                format_for_each_binding_pattern(f, *pattern)?;
            }
            // bare assignment binding
            else {
                write!(f, [pattern])?;
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
            Keyword::Of,
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
    f: &mut TsppFormatter<'ast, '_>,
    initialization: Option<LocalNodeId<Expression>>,
    condition: Option<LocalNodeId<Expression>>,
    increment: Option<LocalNodeId<Expression>>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    let head = format_with(|f| {
        write!(
            f,
            [
                group(&initialization),
                token(";"),
                soft_line_break_or_space(),
                group(&condition),
                token(";"),
                soft_line_break_or_space(),
                group(&increment),
                format_with(|f| write_comments_for_empty_statement_body(f, body)),
            ]
        )
    });

    // grouped header
    write!(f, [Keyword::For, space(), token("(")])?;
    write_grouped_control_head(f, &head)?;
    write!(f, [token(")")])?;

    // body
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }

    format_statement_body_block(f, body)
}

/// Format a `loop` expression.
pub(crate) fn format_loop_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(f, [Keyword::Loop])?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)
}

/// Write a catch or finally keyword after its leading comments.
fn write_try_clause_keyword<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    keyword: Keyword,
    start: u32,
) -> FormatResult<()> {
    let comments = f.context().comments().comments_before(start).to_vec();
    if comments.is_empty() {
        write!(f, [space()])?;
    } else {
        write!(
            f,
            [
                hard_line_break(),
                FormatLeadingComments::Comments(&comments)
            ]
        )?;
    }

    write!(f, [keyword])
}

/// Write one catch parameter inside parentheses.
fn write_catch_parameter<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
    let content = format_with(move |f: &mut TsppFormatter<'ast, '_>| {
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
    f: &mut TsppFormatter<'ast, '_>,
    body: LocalNodeId<Expression>,
    catch: Option<LocalNodeId<Catch>>,
    finally: Option<LocalNodeId<Expression>>,
    force_expanded_branches: bool,
) -> FormatResult<()> {
    // try block
    write!(f, [Keyword::Try])?;
    write_try_branch_after_keyword(f, body, force_expanded_branches)?;

    // catch block
    if let Some(catch) = catch {
        let start = f.context().node_token_start(catch);
        write_try_clause_keyword(f, Keyword::Catch, start)?;
        let catch = f.context().tree.get(catch);
        if let Some(catch_pattern) = catch.pattern {
            write!(f, [space()])?;
            write_catch_parameter(f, catch_pattern, catch.ty)?;
        }
        write_try_branch_after_keyword(f, catch.body, force_expanded_branches)?;
    }

    // finally block
    if let Some(finally) = finally {
        let start = f.context().node_token_start(finally);
        let keyword =
            f.context()
                .token_before_token_start(start)
                .ok_or(FormatError::SyntaxError {
                    message: "missing finally keyword",
                })?;
        write_try_clause_keyword(f, Keyword::Finally, keyword.span.start)?;
        write_try_branch_after_keyword(f, finally, force_expanded_branches)?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Catch> for Catch {
    fn format_node(
        &self,
        _node_id: LocalNodeId<Catch>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Catch])?;
        if let Some(pattern) = self.pattern {
            write!(f, [space()])?;
            write_catch_parameter(f, pattern, self.ty)?;
        }

        write_try_branch_after_keyword(f, self.body, true)
    }
}

/// Write one try, catch, or finally branch after its keyword.
fn write_try_branch_after_keyword<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    branch_expression_id: LocalNodeId<Expression>,
    force_expanded_body: bool,
) -> FormatResult<()> {
    if let Expression::Block(block_id) = f.context().tree.get(branch_expression_id) {
        if force_expanded_body {
            write!(f, [space()])?;
            return format_block_wide(f, *block_id);
        }

        if statement_body_requires_head_space(f.context(), *block_id) {
            write!(f, [space()])?;
        }
    } else {
        write!(f, [space()])?;
    }

    write!(f, [branch_expression_id])
}

/// Format one match arm.
fn format_match_arm<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    arm_id: LocalNodeId<MatchArm>,
) -> FormatResult<()> {
    let arm = f.context().tree.get(arm_id);

    // arm prefix
    write_case_prefix(f, arm_id)?;

    // write the pattern and guard
    let pattern = arm.pattern();
    write!(f, [pattern])?;
    if let Some(guard) = arm.guard() {
        write_match_guard(f, arm_id, pattern, guard)?;
    }

    // arm body
    match arm {
        MatchArm::Expression { body, .. } => {
            write!(f, [space(), token("=>"), space(), *body])?;
        }
        MatchArm::Block { body, .. } => {
            write!(f, [space(), token("=>"), space(), *body])?;
        }
    }

    // arm postfix
    write!(f, [infix_or_postfix_annotations(f.context(), arm_id)])?;

    Ok(())
}

impl<'ast> FormatNode<'ast, MatchArm> for MatchArm {
    fn format_node(
        &self,
        node_id: LocalNodeId<MatchArm>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_match_arm(f, node_id)
    }
}

/// Format one switch case.
fn format_switch_case<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    case_id: LocalNodeId<SwitchCase>,
) -> FormatResult<()> {
    let case = f.context().tree.get(case_id);
    write_case_prefix(f, case_id)?;

    // write the selector
    match case.selector {
        SwitchSelector::Case(value) => {
            write!(f, [Keyword::Case, space(), value, token(":")])?;
        }
        SwitchSelector::Default => {
            write!(f, [Keyword::Default, token(":")])?;
        }
    }

    // keep a sole explicit block beside the selector
    let body = f.context().tree.get(case.body);
    let explicit_block = body.only_expression().filter(|expression| {
        matches!(
            f.context().tree.get(*expression),
            Expression::Block(block) if f.context().tree.get(*block).is_explicit()
        )
    });
    if let Some(explicit_block) = explicit_block {
        write!(f, [space(), explicit_block])?;
    } else if !body.is_empty() {
        write!(
            f,
            [
                hard_line_break(),
                block_indent(&block_statement_sequence(case.body, false, None))
            ]
        )?;
    }

    write!(f, [infix_or_postfix_annotations(f.context(), case_id)])?;

    Ok(())
}

impl<'ast> FormatNode<'ast, SwitchCase> for SwitchCase {
    fn format_node(
        &self,
        node_id: LocalNodeId<SwitchCase>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_switch_case(f, node_id)
    }
}

/// Format one match expression.
pub(crate) fn format_match_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    value: LocalNodeId<Expression>,
    arms: &[LocalNodeId<MatchArm>],
) -> FormatResult<()> {
    write!(f, [Keyword::Match, space()])?;
    write!(
        f,
        [
            token("("),
            format_with(|f| write_grouped_control_head(f, &value)),
            token(")")
        ]
    )?;

    // format the arm block
    if arms.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
    } else {
        write!(f, [space(), token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&format_args![block_indent(&format_with(|f| {
                for (index, arm) in arms.iter().enumerate() {
                    if index > 0 {
                        write!(f, [hard_line_break()])?;
                    }
                    format_match_arm(f, *arm)?;
                }
                Ok(())
            })),])]
        )?;
        write!(f, [block_infix_annotations(f.context(), node_id)])?;
        write!(f, [hard_line_break(), token("}")])?;
    }
    write!(f, [postfix_annotations(f.context(), node_id)])?;

    Ok(())
}

/// Format one switch statement.
pub(crate) fn format_switch_statement<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    value: LocalNodeId<Expression>,
    cases: &[LocalNodeId<SwitchCase>],
) -> FormatResult<()> {
    write!(f, [Keyword::Switch, space()])?;
    write!(
        f,
        [
            token("("),
            format_with(|f| write_grouped_control_head(f, &value)),
            token(")")
        ]
    )?;

    // format the case block
    if cases.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
    } else {
        write!(f, [space(), token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&format_args![block_indent(&format_with(|f| {
                for (index, case) in cases.iter().enumerate() {
                    if index > 0 {
                        write!(f, [hard_line_break()])?;
                    }
                    format_switch_case(f, *case)?;
                }
                Ok(())
            })),])]
        )?;
        write!(f, [block_infix_annotations(f.context(), node_id)])?;
        write!(f, [hard_line_break(), token("}")])?;
    }
    write!(f, [postfix_annotations(f.context(), node_id)])?;

    Ok(())
}
