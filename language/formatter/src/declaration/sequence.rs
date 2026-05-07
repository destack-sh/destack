use std::borrow::Cow;

use crate::annotation::{
    FormatLeadingComments, block_infix_annotations, format_comment, infix_or_postfix_annotations,
    prefix_comment_nodes, statement_prefix_annotations, write_annotation_sequence,
};
use crate::declaration::statement::{
    block_leading_line_comment_nodes, block_trailing_comment_nodes,
};
use crate::declaration::{
    expression_needs_statement_terminator, statement_trailing_comment_anchor_end,
    write_statement_terminator, write_statement_terminator_with_following_start,
};
use crate::expression::format_expression;
use crate::file::{
    ignore_range_for_node, ignore_ranges_for_nodes, node_has_ignore_directive, write_ignored_span,
};
use destack_ast::{
    Block, BlockContext, Comment, Declaration, DecoratorPosition, Expression, FunctionForm,
    FunctionRole, FunctionSignature, IfForm, LocalNodeId, Member, NodeType, Property, TokenSpan,
    Tree, TypeExpression, TypeLiteral,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::{format_with, *};
use destack_fir::{format_args, write};
use destack_source::{FileId, LanguageType, NodeSpanRegion, NodeSpanType, Span};

use crate::declaration::dependency as imports;
use crate::{DestackFormatContext, DestackFormatter};

/// Return the statement source span for one expression.
fn expression_statement_span(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Span {
    let expression_span = context.span(expression_id);

    context
        .tree
        .get_side_span(
            expression_id,
            NodeSpanType::Region(NodeSpanRegion::Statement),
        )
        .unwrap_or(expression_span)
}

/// Return whether one expression has a source blank line before it.
fn expression_has_lines_before(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_span = expression_statement_span(context, expression_id);
    let expression_start = expression_prefix_start(context, expression_id, expression_span.start);
    let expression_span = Span::new(expression_span.file, expression_start, expression_span.end);

    context
        .source_text()
        .get_lines_before(expression_span, context.comments())
        > 1
}

/// Format one program-scoped statement sequence.
pub(crate) fn program_statement_sequence<'ast>(
    expressions: &'ast [LocalNodeId<Expression>],
) -> impl Format<DestackFormatContext<'ast>> + 'ast {
    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        format_program_statement_sequence(f, expressions)
    })
}

/// Format one block-scoped statement sequence.
pub(crate) fn block_statement_sequence<'ast>(
    block_id: LocalNodeId<Block>,
    allow_value_tail: bool,
    leading_prefix_comment_start: Option<u32>,
) -> impl Format<DestackFormatContext<'ast>> + 'ast {
    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        format_block_statement_sequence_for_block(
            f,
            block_id,
            allow_value_tail,
            leading_prefix_comment_start,
        )
    })
}

/// Return whether trivia between two offsets contains an explicit blank line.
fn has_blank_line_between_offsets(
    context: &DestackFormatContext<'_>,
    file: FileId,
    start: u32,
    end: u32,
) -> bool {
    if end <= start {
        return false;
    }

    let between_span = Span::new(file, start, end);
    context.has_blank_line(between_span)
}

/// Return the earliest start offset for leading comments on an expression.
fn expression_prefix_start(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    default_start: u32,
) -> u32 {
    let mut start = default_start;

    for comment in prefix_comment_nodes(context, expression_id) {
        start = start.min(comment.span.start);
    }

    // declaration expressions semantically start at their prefix annotations,
    // even though the expression span begins at the declaration head
    if let Expression::Declaration(declaration_id) = context.tree.get(expression_id) {
        for comment in prefix_comment_nodes(context, *declaration_id) {
            start = start.min(comment.span.start);
        }

        for annotation_id in context.annotation_ids(*declaration_id).iter().copied() {
            if matches!(
                context.annotation(annotation_id).position,
                DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
            ) {
                start = start.min(context.annotation_span(annotation_id).start);
            }
        }
    }

    for annotation_id in context.annotation_ids(expression_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position,
            DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
        ) {
            start = start.min(context.annotation_span(annotation_id).start);
        }
    }

    let semantic_head_start = match context.tree.get(expression_id) {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Call { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => {
            let expression_span = context.span(*left);
            expression_prefix_start(context, *left, expression_span.start)
        }
        _ => default_start,
    };

    start.min(semantic_head_start)
}

/// Return the stateless source start for following sibling trivia boundaries.
fn expression_following_span_start(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression_span = expression_statement_span(context, expression_id);
    let mut start = expression_span.start;

    if let Expression::Declaration(declaration_id) = context.tree.get(expression_id) {
        for annotation_id in context.annotation_ids(*declaration_id).iter().copied() {
            if matches!(
                context.annotation(annotation_id).position,
                DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
            ) {
                start = start.min(context.annotation_span(annotation_id).start);
            }
        }
    }

    for annotation_id in context.annotation_ids(expression_id).iter().copied() {
        if matches!(
            context.annotation(annotation_id).position,
            DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
        ) {
            start = start.min(context.annotation_span(annotation_id).start);
        }
    }

    start
}

/// Return the latest end offset for trailing comments on an expression.
pub(crate) fn expression_postfix_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    default_end: u32,
) -> u32 {
    let default_end =
        statement_trailing_comment_anchor_end(context, expression_id).max(default_end);

    context
        .end_of_line_comment_tokens_after(default_end)
        .iter()
        .fold(default_end, |end, comment| end.max(comment.span.end))
}

/// Skip a source semicolon and horizontal trivia after one formatted statement end.
fn advance_past_source_statement_terminator(
    context: &DestackFormatContext<'_>,
    mut offset: u32,
) -> u32 {
    // skip the source semicolon that the formatter already re-emitted
    while let Some(byte) = context.source_text().byte_at(offset) {
        if matches!(byte, b';' | b' ' | b'\t') {
            offset += 1;
            continue;
        }

        break;
    }

    offset
}

/// Return the raw prefix start before one ignored range.
fn ignored_range_prefix_start(
    context: &DestackFormatContext<'_>,
    previous_output_end: Option<(FileId, u32)>,
    previous_output_was_ignored: bool,
    previous_expression_id: Option<LocalNodeId<Expression>>,
    expression_id: LocalNodeId<Expression>,
    expression_span: Span,
) -> u32 {
    // stay on the formatter-managed source cursor when the previous output was formatted
    if let Some((previous_file, previous_end)) = previous_output_end
        && previous_file == expression_span.file
    {
        if previous_output_was_ignored {
            return previous_end;
        }

        return advance_past_source_statement_terminator(context, previous_end);
    }

    // otherwise fall back to the previous source statement end in the same file
    if let Some(previous_expression_id) = previous_expression_id {
        let previous_span = context.span(previous_expression_id);

        if previous_span.file == expression_span.file {
            return advance_past_source_statement_terminator(context, previous_span.end);
        }
    }

    expression_prefix_start(context, expression_id, expression_span.start)
}

/// Return comments between one previous statement end and the next expression head.
fn expression_gap_comment_nodes(
    context: &DestackFormatContext<'_>,
    start: u32,
    expression_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let expression_span = context.span(expression_id);
    let expression_start = expression_prefix_start(context, expression_id, expression_span.start);
    if expression_start <= start {
        return Vec::new();
    }

    {
        let comments = context.comments();
        comments.comments_in_range(start, expression_start).to_vec()
    }
}

/// Write statement-gap comments before one expression head.
fn write_expression_gap_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    start: u32,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let comment_nodes = expression_gap_comment_nodes(f.context(), start, expression_id);
    if comment_nodes.is_empty() {
        return Ok(());
    }

    write!(f, [FormatLeadingComments::Comments(&comment_nodes)])
}

/// Write one comment node sequence separated by hard line breaks.
fn write_comment_node_lines<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comment_nodes: &[Comment],
) -> FormatResult<()> {
    for (index, comment) in comment_nodes.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        format_comment(f, comment)?;
    }

    Ok(())
}

/// Write postfix annotations for one block expression.
fn write_expression_postfix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
    is_ignored: bool,
    _use_statement_inner_annotations: bool,
) -> FormatResult<()> {
    let if_chain_handles_annotations = matches!(
        expression,
        Expression::If {
            form: IfForm::If,
            ..
        }
    );
    if if_chain_handles_annotations || is_ignored {
        return Ok(());
    }

    write!(
        f,
        [infix_or_postfix_annotations(f.context(), expression_id)]
    )
}

/// Return whether one expression is a lambda declaration expression.
fn expression_is_lambda_declaration(tree: &Tree, expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Declaration(declaration_id)
            if matches!(
                tree.get(*declaration_id),
                Declaration::Function(function)
                    if function.signature.form == FunctionForm::Lambda
            )
    )
}

/// Write prefix annotations for one statement-sequence expression.
fn write_statement_sequence_expression_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
    start_offset: Option<u32>,
) -> FormatResult<()> {
    if matches!(expression, Expression::Declaration(_)) {
        return Ok(());
    }

    if expression_is_lambda_declaration(f.context().tree, expression) {
        let mut prefix_items = Vec::new();

        for annotation_id in f.context().annotation_ids(expression_id).iter().copied() {
            if f.context().annotation(annotation_id).position == DecoratorPosition::BlockPrefix {
                prefix_items.push(annotation_id);
            }
        }

        return write_annotation_sequence(f, &prefix_items);
    }

    write!(
        f,
        [statement_prefix_annotations(
            f.context(),
            expression_id,
            start_offset
        )]
    )
}

/// Format one statement-sequence expression and return the rendered end offset.
fn format_statement_sequence_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
    is_ignored: bool,
    allow_value_tail: bool,
    is_expression_context_tail: bool,
    prefix_after_offset: Option<u32>,
    following_expression_start: Option<u32>,
) -> FormatResult<u32> {
    write_statement_sequence_expression_prefix(f, expression_id, expression, prefix_after_offset)?;

    let expression_is_value_tail = allow_value_tail && is_expression_context_tail;
    let needs_statement_parentheses =
        statement_sequence_expression_needs_parentheses(expression, expression_is_value_tail);
    if needs_statement_parentheses {
        write!(f, [token("(")])?;
    }

    format_expression(f, expression_id, expression, is_ignored)?;

    if needs_statement_parentheses {
        write!(f, [token(")")])?;
    }

    if expression_needs_statement_terminator(
        f.context(),
        expression,
        allow_value_tail && is_expression_context_tail,
    ) {
        if let Some(following_expression_start) = following_expression_start {
            let anchor_end = statement_trailing_comment_anchor_end(f.context(), expression_id);
            write_statement_terminator_with_following_start(
                f,
                expression_id,
                anchor_end,
                following_expression_start,
            )?;
        } else {
            write_statement_terminator(f, expression_id)?;
        }
    }

    write_expression_postfix_annotations(f, expression_id, expression, is_ignored, false)?;

    let expression_span = f.context().span(expression_id);
    Ok(expression_postfix_end(
        f.context(),
        expression_id,
        expression_span.end,
    ))
}

/// Return whether one statement-sequence expression needs disambiguating parentheses.
fn statement_sequence_expression_needs_parentheses(
    expression: &Expression,
    is_value_tail: bool,
) -> bool {
    if is_value_tail {
        return false;
    }

    matches!(expression, Expression::ObjectExpression { ty: None, .. })
}

/// Format a block inline with zero or one expression (including label and infix annotations).
/// Format block contents with compact inner spacing.
#[inline]
pub(crate) fn format_block_body_narrow<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    debug_assert!(block.len() <= 1);

    // body
    if block.is_empty() {
        write!(f, [token("{"), token("}")])?;
    } else {
        let expression_id = block.first_expression().expect("single-expression block");
        let expression = f.context().tree.get(expression_id);
        let allow_value_tail = block_allows_value_tail(f.context(), block_id);
        let is_expression_context_tail = block
            .tail_expression
            .is_some_and(|tail_expression_id| tail_expression_id == expression_id);
        let body = format_with(|f| {
            format_statement_sequence_expression(
                f,
                expression_id,
                expression,
                node_has_ignore_directive(f.context(), expression_id),
                allow_value_tail,
                is_expression_context_tail,
                None,
                None,
            )
            .map(|_| ())
        });

        write!(
            f,
            [
                token("{"),
                soft_line_break_or_space(),
                soft_block_indent(&format_args![
                    &body,
                    block_infix_annotations(f.context(), block_id)
                ]),
                soft_line_break_or_space(),
                token("}")
            ]
        )?;
    }
    Ok(())
}

/// Format a block multiline with multiple expressions (including label and infix annotations).
/// Format block contents with expanded inner spacing.
#[inline]
pub(crate) fn format_block_body_wide<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    let allow_value_tail = block_allows_value_tail(f.context(), block_id);
    let leading_comment_nodes = block_leading_line_comment_nodes(f.context(), block_id);
    let has_infix_annotation = f.context().has_infix_annotation(block_id);

    // body
    write!(f, [token("{"), hard_line_break()])?;

    if !leading_comment_nodes.is_empty() {
        write!(
            f,
            [block_indent(&format_with(
                |f: &mut DestackFormatter<'ast, '_>| {
                    write_comment_node_lines(f, &leading_comment_nodes)
                }
            ))]
        )?;
        if !block.is_empty() || has_infix_annotation {
            write!(f, [hard_line_break()])?;
        }
    }

    if !block.is_empty() {
        let leading_prefix_comment_start =
            leading_comment_nodes.last().map(|comment| comment.span.end);
        write!(
            f,
            [soft_block_indent(&block_statement_sequence(
                block_id,
                allow_value_tail,
                leading_prefix_comment_start
            ))]
        )?;
    }

    let trailing_comment_nodes = block_trailing_comment_nodes(f.context(), block_id);

    if !block.is_empty() && (!trailing_comment_nodes.is_empty() || has_infix_annotation) {
        write!(f, [hard_line_break()])?;
    }

    if !trailing_comment_nodes.is_empty() {
        write!(
            f,
            [block_indent(&format_with(
                |f: &mut DestackFormatter<'ast, '_>| {
                    write_comment_node_lines(f, &trailing_comment_nodes)
                }
            ))]
        )?;
        if has_infix_annotation {
            write!(f, [hard_line_break()])?;
        }
    }

    // infix annotations
    if has_infix_annotation {
        write!(
            f,
            [block_indent(&block_infix_annotations(
                f.context(),
                block_id
            ))]
        )?;
    }

    write!(f, [hard_line_break(), token("}")])
}

/// Collect ignore ranges for block expressions without building another list.
fn ignore_ranges_for_block_expressions(
    ctx: &DestackFormatContext<'_>,
    block: &Block,
    comment_tokens: &[TokenSpan],
) -> std::collections::HashMap<u32, Span> {
    let mut ignore_ranges = std::collections::HashMap::new();

    for expression_id in block.iter_expressions() {
        if let Some(range_span) = ignore_range_for_node(ctx, expression_id, comment_tokens) {
            ignore_ranges.insert(expression_id.id, range_span);
        }
    }

    ignore_ranges
}

/// Format one block statement sequence with spacing and ignore handling.
pub(crate) fn format_block_statement_sequence<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expressions: &[LocalNodeId<Expression>],
    allow_value_tail: bool,
) -> FormatResult<()> {
    // ignore ranges: only compute when the file may contain ignore directives
    let ignore_ranges = if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        ignore_ranges_for_nodes(f.context(), expressions, comment_tokens)
    } else {
        std::collections::HashMap::new()
    };
    let effective_expressions = expressions;
    let mut previous_output_end: Option<(FileId, u32)> = None;
    let mut previous_output_was_ignored = false;
    let mut skip_until: Option<u32> = None;

    for (i, &expression_id) in effective_expressions.iter().enumerate() {
        let expression = f.context().tree.get(expression_id);
        let ignore_range = ignore_ranges.get(&expression_id.id).copied();
        let has_ignore_range = ignore_range.is_some();
        let is_ignored = node_has_ignore_directive(f.context(), expression_id);

        let expression_span = f.context().span(expression_id);

        if let Some(skip_end) = skip_until {
            if expression_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between expressions
        if i > 0 {
            let previous_expression_id = effective_expressions[i - 1];
            let source_has_blank_line_between = if has_ignore_range {
                if let Some((previous_file, previous_end)) = previous_output_end {
                    if previous_file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_end,
                            range_start,
                        )
                    }
                } else {
                    let previous_span = f.context().span(previous_expression_id);
                    if previous_span.file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_span.end,
                            range_start,
                        )
                    }
                }
            } else {
                expression_has_lines_before(f.context(), expression_id)
            };
            if !has_ignore_range {
                if !source_has_blank_line_between {
                    write!(f, [hard_line_break()])?;
                }

                // determine if we need an extra blank line
                let needs_blank = source_has_blank_line_between;

                if needs_blank {
                    write!(f, [empty_line()])?;
                }
            }
        }

        if let Some(range_span) = ignore_range {
            // the first ignored statement has no previous source sibling
            let previous_expression_id = if i > 0 {
                Some(effective_expressions[i - 1])
            } else {
                None
            };

            let prefix_start = ignored_range_prefix_start(
                f.context(),
                previous_output_end,
                previous_output_was_ignored,
                previous_expression_id,
                expression_id,
                expression_span,
            );
            if prefix_start < range_span.start {
                let prefix_span = Span::new(range_span.file, prefix_start, range_span.start);
                write_ignored_span(f, prefix_span)?;
            }

            write_ignored_span(f, range_span)?;
            skip_until = Some(range_span.end);
            previous_output_end = Some((range_span.file, range_span.end));
            previous_output_was_ignored = true;
            continue;
        }

        // separator comments
        if i > 0 {
            let gap_start = previous_output_end
                .filter(|(file, _)| *file == expression_span.file)
                .map_or_else(
                    || f.context().span(effective_expressions[i - 1]).end,
                    |(_, previous_end)| previous_end,
                );
            write_expression_gap_comments(f, gap_start, expression_id)?;
        }

        let is_expression_context_tail = allow_value_tail && i + 1 == effective_expressions.len();
        let following_expression_start = effective_expressions
            .get(i + 1)
            .map(|expression_id| expression_following_span_start(f.context(), *expression_id));
        let expression_output_end = format_statement_sequence_expression(
            f,
            expression_id,
            expression,
            is_ignored,
            allow_value_tail,
            is_expression_context_tail,
            None,
            following_expression_start,
        )?;

        f.context_mut()
            .comments_mut()
            .skip_comments_before(expression_span.end);

        previous_output_end = Some((expression_span.file, expression_output_end));
        previous_output_was_ignored = false;
    }
    Ok(())
}

/// Format one block statement sequence with spacing and ignore handling.
pub(crate) fn format_block_statement_sequence_for_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
    allow_value_tail: bool,
    leading_prefix_comment_start: Option<u32>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    let expressions: Vec<_> = block.iter_expressions().collect();

    // ignore ranges: only compute when the file may contain ignore directives
    let ignore_ranges = if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        ignore_ranges_for_block_expressions(f.context(), block, comment_tokens)
    } else {
        std::collections::HashMap::new()
    };
    let has_ignore_ranges = !ignore_ranges.is_empty();
    let tree = f.context().tree;
    let strings = f.context().strings;

    // block body source cursor
    let block_body_start = f
        .context()
        .first_non_trivia_token_in_span(f.context().span(block_id))
        .map(|token| (token.span.file, token.span.end));

    // import section at the block head
    let import_count = expressions
        .iter()
        .take_while(|&&expr_id| imports::is_import(expr_id, tree))
        .count();

    // organized import section when enabled
    let sorted_imports: Vec<LocalNodeId<Expression>>;
    let effective_expressions: Cow<'_, [LocalNodeId<Expression>]> =
        if f.context().options.organize_imports.is_enabled()
            && import_count > 1
            && !has_ignore_ranges
        {
            sorted_imports = imports::sort_imports(&expressions[..import_count], tree, strings);
            Cow::Owned(
                sorted_imports
                    .iter()
                    .copied()
                    .chain(expressions[import_count..].iter().copied())
                    .collect(),
            )
        } else {
            Cow::Borrowed(&expressions)
        };

    let mut prev_was_import = false;
    let mut prev_import_id: Option<LocalNodeId<Expression>> = None;
    let mut previous_output_end = block_body_start;
    let mut previous_output_was_ignored = false;
    let mut previous_expression_id: Option<LocalNodeId<Expression>> = None;
    let mut skip_until: Option<u32> = None;

    for (i, expression_id) in effective_expressions.iter().copied().enumerate() {
        let expression = f.context().tree.get(expression_id);
        let is_import_expr = imports::is_import(expression_id, tree);
        let ignore_range = ignore_ranges.get(&expression_id.id).copied();
        let has_ignore_range = ignore_range.is_some();
        let is_ignored = node_has_ignore_directive(f.context(), expression_id);

        let expression_span = f.context().span(expression_id);

        if let Some(skip_end) = skip_until {
            if expression_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between expressions
        if let Some(previous_expression_id) = previous_expression_id {
            let source_has_blank_line_between = if has_ignore_range {
                if let Some((previous_file, previous_end)) = previous_output_end {
                    if previous_file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_end,
                            range_start,
                        )
                    }
                } else {
                    let previous_span = f.context().span(previous_expression_id);
                    if previous_span.file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_span.end,
                            range_start,
                        )
                    }
                }
            } else {
                expression_has_lines_before(f.context(), expression_id)
            };
            if !has_ignore_range {
                if !source_has_blank_line_between {
                    write!(f, [hard_line_break()])?;
                }

                // import section spacing and explicit source blank lines
                let needs_blank = if f.context().options.organize_imports.is_enabled()
                    && prev_was_import
                    && is_import_expr
                {
                    prev_import_id.is_some_and(|prev_id| {
                        imports::should_insert_blank_between(
                            prev_id,
                            expression_id,
                            f.context().tree,
                            f.context().strings,
                        )
                    })
                } else {
                    source_has_blank_line_between
                };

                if needs_blank {
                    write!(f, [empty_line()])?;
                }
            }
        }

        if let Some(range_span) = ignore_range {
            let prefix_start = ignored_range_prefix_start(
                f.context(),
                previous_output_end,
                previous_output_was_ignored,
                previous_expression_id,
                expression_id,
                expression_span,
            );
            if prefix_start < range_span.start {
                let prefix_span = Span::new(range_span.file, prefix_start, range_span.start);
                write_ignored_span(f, prefix_span)?;
            }

            write_ignored_span(f, range_span)?;
            skip_until = Some(range_span.end);
            prev_was_import = false;
            prev_import_id = None;
            previous_output_end = Some((range_span.file, range_span.end));
            previous_output_was_ignored = true;
            continue;
        }

        // separator comments
        if previous_expression_id.is_some() || previous_output_end.is_some() {
            let gap_start = previous_output_end
                .filter(|(file, _)| *file == expression_span.file)
                .map_or_else(
                    || f.context().span(effective_expressions[i - 1]).end,
                    |(_, previous_end)| previous_end,
                );
            write_expression_gap_comments(f, gap_start, expression_id)?;
        }

        let is_expression_context_tail = allow_value_tail
            && block
                .tail_expression
                .is_some_and(|tail| tail == expression_id);
        let prefix_after_offset = if previous_expression_id.is_none() {
            leading_prefix_comment_start
        } else {
            None
        };
        let following_expression_start = effective_expressions
            .get(i + 1)
            .map(|expression_id| expression_following_span_start(f.context(), *expression_id));
        let expression_output_end = format_statement_sequence_expression(
            f,
            expression_id,
            expression,
            is_ignored,
            allow_value_tail,
            is_expression_context_tail,
            prefix_after_offset,
            following_expression_start,
        )?;

        f.context_mut()
            .comments_mut()
            .skip_comments_before(expression_output_end);

        prev_was_import = is_import_expr;
        if is_import_expr {
            prev_import_id = Some(expression_id);
        } else {
            prev_import_id = None;
        }

        previous_output_end = Some((expression_span.file, expression_output_end));
        previous_output_was_ignored = false;
        previous_expression_id = Some(expression_id);
    }
    Ok(())
}

/// Format one program statement sequence with spacing, import organization, and ignore handling.
fn format_program_statement_sequence<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    let tree = f.context().tree;
    let strings = f.context().strings;

    // ignore ranges: only compute when the file may contain ignore directives
    let ignore_ranges = if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        ignore_ranges_for_nodes(f.context(), expressions, comment_tokens)
    } else {
        std::collections::HashMap::new()
    };
    let has_ignore_ranges = !ignore_ranges.is_empty();

    // import section at the program head
    let import_count = expressions
        .iter()
        .take_while(|&&expr_id| imports::is_import(expr_id, tree))
        .count();

    // organized import section when enabled
    let sorted_imports: Vec<LocalNodeId<Expression>>;
    let effective_expressions: Cow<'_, [LocalNodeId<Expression>]> =
        if f.context().options.organize_imports.is_enabled()
            && import_count > 1
            && !has_ignore_ranges
        {
            sorted_imports = imports::sort_imports(&expressions[..import_count], tree, strings);
            Cow::Owned(
                sorted_imports
                    .iter()
                    .copied()
                    .chain(expressions[import_count..].iter().copied())
                    .collect(),
            )
        } else {
            Cow::Borrowed(expressions)
        };

    let mut prev_was_import = false;
    let mut prev_import_id: Option<LocalNodeId<Expression>> = None;
    let mut previous_output_end: Option<(FileId, u32)> = None;
    let mut previous_output_was_ignored = false;
    let mut skip_until: Option<u32> = None;

    for (i, &expression_id) in effective_expressions.iter().enumerate() {
        let expression = f.context().tree.get(expression_id);
        let is_import_expr = imports::is_import(expression_id, tree);
        let ignore_range = ignore_ranges.get(&expression_id.id).copied();
        let has_ignore_range = ignore_range.is_some();
        let is_ignored = node_has_ignore_directive(f.context(), expression_id);

        let expression_span = f.context().span(expression_id);

        if let Some(skip_end) = skip_until {
            if expression_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between expressions
        if i > 0 {
            let previous_expression_id = effective_expressions[i - 1];
            let source_has_blank_line_between = if has_ignore_range {
                if let Some((previous_file, previous_end)) = previous_output_end {
                    if previous_file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_end,
                            range_start,
                        )
                    }
                } else {
                    let previous_span = f.context().span(previous_expression_id);
                    if previous_span.file != expression_span.file {
                        false
                    } else {
                        let range_start =
                            ignore_range.map_or(expression_span.start, |span| span.start);
                        has_blank_line_between_offsets(
                            f.context(),
                            expression_span.file,
                            previous_span.end,
                            range_start,
                        )
                    }
                }
            } else {
                expression_has_lines_before(f.context(), expression_id)
            };
            if !has_ignore_range {
                write!(f, [hard_line_break()])?;

                // import section spacing and explicit source blank lines
                let needs_blank = if f.context().options.organize_imports.is_enabled()
                    && prev_was_import
                    && is_import_expr
                {
                    prev_import_id.is_some_and(|prev_id| {
                        imports::should_insert_blank_between(
                            prev_id,
                            expression_id,
                            f.context().tree,
                            f.context().strings,
                        )
                    })
                } else {
                    source_has_blank_line_between
                };

                if needs_blank {
                    write!(f, [empty_line()])?;
                }
            }
        }

        if let Some(range_span) = ignore_range {
            // the first ignored statement has no previous source sibling
            let previous_expression_id = if i > 0 {
                Some(effective_expressions[i - 1])
            } else {
                None
            };

            let prefix_start = ignored_range_prefix_start(
                f.context(),
                previous_output_end,
                previous_output_was_ignored,
                previous_expression_id,
                expression_id,
                expression_span,
            );
            if prefix_start < range_span.start {
                let prefix_span = Span::new(range_span.file, prefix_start, range_span.start);
                write_ignored_span(f, prefix_span)?;
            }

            write_ignored_span(f, range_span)?;
            skip_until = Some(range_span.end);
            prev_was_import = false;
            prev_import_id = None;
            previous_output_end = Some((range_span.file, range_span.end));
            previous_output_was_ignored = true;
            continue;
        }

        // raw file header comments
        if i == 0 && previous_output_end.is_none() {
            write_expression_gap_comments(f, 0, expression_id)?;
        } else if i > 0 {
            // separator comments
            let gap_start = previous_output_end
                .filter(|(file, _)| *file == expression_span.file)
                .map_or_else(
                    || f.context().span(effective_expressions[i - 1]).end,
                    |(_, previous_end)| previous_end,
                );
            write_expression_gap_comments(f, gap_start, expression_id)?;
        }

        prev_was_import = is_import_expr;
        if is_import_expr {
            prev_import_id = Some(expression_id);
        }
        let expression_output_end = format_statement_sequence_expression(
            f,
            expression_id,
            expression,
            is_ignored,
            false,
            false,
            None,
            effective_expressions
                .get(i + 1)
                .map(|expression_id| f.context().span(*expression_id).start),
        )?;

        f.context_mut()
            .comments_mut()
            .skip_comments_before(expression_output_end);

        previous_output_end = Some((expression_span.file, expression_output_end));
        previous_output_was_ignored = false;
    }

    Ok(())
}

/// Return true when the final expression in this block is value-position.
pub(crate) fn block_allows_value_tail(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    if block.context != BlockContext::Expression {
        return false;
    }

    let Some((block_expression_id, block_expression_type)) = context.parent(block_id) else {
        return false;
    };
    if block_expression_type == NodeType::MatchCase {
        return true;
    }
    if block_expression_type != NodeType::Expression {
        return false;
    }

    let block_expression_id = LocalNodeId::<Expression>::new(block_expression_id);
    let Expression::Block(inner_block_id) = context.tree.get(block_expression_id) else {
        return false;
    };
    if *inner_block_id != block_id {
        return false;
    }

    !expression_is_in_statement_position(context, block_expression_id)
}

/// Return true when this expression is the value tail of one block.
pub(crate) fn expression_is_value_block_tail(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return false;
    };
    if parent_type != NodeType::Block {
        return false;
    }

    let block_id = LocalNodeId::<Block>::new(parent_id);
    let block = context.tree.get(block_id);
    if block.tail_expression != Some(expression_id) {
        return false;
    }

    block_allows_value_tail(context, block_id)
}

/// Return true when this expression is in statement position.
pub(crate) fn expression_is_in_statement_position(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return true;
    };

    match parent_type {
        NodeType::Expression => {
            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            let parent_expression = context.tree.get(parent_expression_id);
            if parent_expression_id == expression_id {
                return true;
            }

            let should_inherit_parent_position = match parent_expression {
                Expression::Parenthesized { expression } => expression.id == expression_id.id,
                Expression::If {
                    then_expression,
                    else_expression,
                    ..
                } => {
                    if !control_branch_inherits_statement_position(context) {
                        return false;
                    }

                    then_expression.id == expression_id.id
                        || else_expression
                            .as_ref()
                            .is_some_and(|else_expression| else_expression.id == expression_id.id)
                }
                Expression::While { body, .. }
                | Expression::ForEach { body, .. }
                | Expression::For { body, .. }
                | Expression::Loop { body } => body.id == expression_id.id,
                Expression::Try {
                    try_expression,
                    catch_expression,
                    finally_expression,
                    ..
                } => {
                    if finally_expression
                        .as_ref()
                        .is_some_and(|finally_expression| finally_expression.id == expression_id.id)
                    {
                        return true;
                    }

                    if !control_branch_inherits_statement_position(context) {
                        return false;
                    }

                    try_expression.id == expression_id.id
                        || catch_expression
                            .as_ref()
                            .is_some_and(|catch_expression| catch_expression.id == expression_id.id)
                }
                Expression::Labelled { body, .. } => body.id == expression_id.id,
                _ => false,
            };

            should_inherit_parent_position
                && expression_is_in_statement_position(context, parent_expression_id)
        }
        NodeType::Block => expression_is_in_statement_position_inside_parent_block(
            context,
            LocalNodeId::<Block>::new(parent_id),
            expression_id,
        ),
        NodeType::Declaration => expression_is_in_statement_position_inside_parent_declaration(
            context,
            LocalNodeId::<Declaration>::new(parent_id),
            expression_id,
        ),
        NodeType::Member => expression_is_in_statement_position_inside_parent_member(
            context,
            LocalNodeId::<Member>::new(parent_id),
            expression_id,
        ),
        NodeType::Property => expression_is_in_statement_position_inside_parent_property(
            context,
            LocalNodeId::<Property>::new(parent_id),
            expression_id,
        ),
        _ => false,
    }
}

/// Return true when value-capable control branches use statement formatting in this language.
fn control_branch_inherits_statement_position(context: &DestackFormatContext<'_>) -> bool {
    !LanguageType::from(context.file.ty).is_destack()
}

/// Return true when one child expression is statement-position inside one parent block.
fn expression_is_in_statement_position_inside_parent_block(
    context: &DestackFormatContext<'_>,
    parent_block_id: LocalNodeId<Block>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_block = context.tree.get(parent_block_id);

    if parent_block.context == BlockContext::Statement {
        return true;
    }

    let is_tail_expression = parent_block
        .tail_expression
        .is_some_and(|tail_expression_id| tail_expression_id == expression_id);
    if !is_tail_expression {
        return true;
    }

    !block_allows_value_tail(context, parent_block_id)
}

/// Return true when one child expression is statement-position inside one parent declaration.
fn expression_is_in_statement_position_inside_parent_declaration(
    context: &DestackFormatContext<'_>,
    parent_declaration_id: LocalNodeId<Declaration>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_declaration = context.tree.get(parent_declaration_id);

    match parent_declaration {
        Declaration::Function(function) => function.body.is_some_and(|body_expression_id| {
            if body_expression_id.id != expression_id.id {
                return false;
            }

            function_body_is_statement_position(context, &function.signature)
        }),
        Declaration::Global(global) => global.expressions.contains(&expression_id),
        Declaration::Namespace(namespace) => namespace.expressions.contains(&expression_id),
        _ => false,
    }
}

/// Return true when one child expression is statement-position inside one parent member.
fn expression_is_in_statement_position_inside_parent_member(
    context: &DestackFormatContext<'_>,
    parent_member_id: LocalNodeId<Member>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_member = context.tree.get(parent_member_id);

    match parent_member {
        Member::Method {
            signature, body, ..
        } => body.as_ref().is_some_and(|body_expression_id| {
            body_expression_id.id == expression_id.id
                && function_body_is_statement_position(context, signature)
        }),
        Member::StaticBlock { body, .. } | Member::ComptimeBlock { body, .. } => {
            body.id == expression_id.id
        }
        _ => false,
    }
}

/// Return true when one child expression is statement-position inside one parent property method.
fn expression_is_in_statement_position_inside_parent_property(
    context: &DestackFormatContext<'_>,
    parent_property_id: LocalNodeId<Property>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_property = context.tree.get(parent_property_id);

    match parent_property {
        Property::Method {
            signature, body, ..
        } => body.as_ref().is_some_and(|body_expression_id| {
            body_expression_id.id == expression_id.id
                && function_body_is_statement_position(context, signature)
        }),
        _ => false,
    }
}

/// Return true when one function-like body should be statement-position.
fn function_body_is_statement_position(
    context: &DestackFormatContext<'_>,
    signature: &FunctionSignature,
) -> bool {
    if matches!(
        signature.role,
        Some(FunctionRole::Constructor | FunctionRole::Setter)
    ) {
        return true;
    }

    signature
        .return_type
        .is_some_and(|return_type| type_expression_is_void(context, return_type))
}

/// Return true when one type expression is exactly `void`.
fn type_expression_is_void(
    context: &DestackFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(type_id) {
        TypeExpression::Parenthesized { expression } => {
            type_expression_is_void(context, *expression)
        }
        TypeExpression::Literal {
            value: TypeLiteral::Void,
        } => true,
        _ => false,
    }
}
