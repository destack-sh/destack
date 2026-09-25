use std::borrow::Cow;
use std::collections::HashMap;

use crate::annotation::{
    FormatLeadingComments, block_infix_annotations, format_comment, infix_or_postfix_annotations,
    prefix_comment_nodes, statement_prefix_annotations, write_annotation_sequence,
};
use crate::declaration::dependency::{is_import, should_insert_blank_between, sort_imports};
use crate::declaration::statement::{
    block_leading_line_comment_nodes, block_trailing_comment_nodes,
};
use crate::declaration::{
    expression_needs_statement_terminator, statement_trailing_comment_anchor_end,
    write_semicolonless_statement_comments, write_statement_terminator,
    write_statement_terminator_with_following_start,
};
use crate::expression::{expression_is_lambda_declaration, format_expression};
use crate::file::{ignore_ranges_for_nodes, node_has_ignore_directive, write_source_span};
use tspp_dir::{
    Block, BlockContext, Comment, Declaration, DecoratorPosition, Expression, FunctionRole,
    FunctionSignature, IfForm, LocalNodeId, Member, NodeType, Property, TypeExpression,
    TypeLiteral,
};
use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::{format_with, *};
use tspp_fir::{format_args, write};
use tspp_source::{FileId, Span};

use crate::{TsppFormatContext, TsppFormatter};

/// Return whether one expression has a source blank line before it.
fn expression_has_lines_before(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_span = context.expression_statement_extent(expression_id);
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
) -> impl Format<'ast, TsppFormatContext<'ast>> + 'ast {
    format_with(move |f: &mut TsppFormatter<'ast, '_>| {
        format_program_statement_sequence(f, expressions)
    })
}

/// Format one block-scoped statement sequence.
pub(crate) fn block_statement_sequence<'ast>(
    block_id: LocalNodeId<Block>,
    allow_value_tail: bool,
    leading_prefix_comment_start: Option<u32>,
) -> impl Format<'ast, TsppFormatContext<'ast>> + 'ast {
    format_with(move |f: &mut TsppFormatter<'ast, '_>| {
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
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
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

        start = context.prefix_annotation_start(*declaration_id, start);
    }

    start = context.prefix_annotation_start(expression_id, start);

    let semantic_head_start = match context.tree.get(expression_id) {
        Expression::Member { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Call { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::Chain { expression: left } => {
            let expression_span = context.span(*left);
            expression_prefix_start(context, *left, expression_span.start)
        }
        _ => default_start,
    };

    start.min(semantic_head_start)
}

/// Return the stateless source start for following sibling trivia boundaries.
fn expression_following_span_start(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression_span = context.expression_statement_extent(expression_id);
    let mut start = expression_span.start;

    if let Expression::Declaration(declaration_id) = context.tree.get(expression_id) {
        start = context.prefix_annotation_start(*declaration_id, start);
    }

    start = context.prefix_annotation_start(expression_id, start);

    start
}

/// Return the latest end offset for trailing comments on an expression.
pub(crate) fn expression_postfix_end(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    default_end: u32,
) -> u32 {
    let default_end =
        statement_trailing_comment_anchor_end(context, expression_id).max(default_end);

    context
        .source_end_of_line_comments_after(default_end)
        .iter()
        .fold(default_end, |end, comment| end.max(comment.span.end))
}

/// Skip a source semicolon and horizontal trivia after one formatted statement end.
fn advance_past_source_statement_terminator(
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
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
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
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

/// Write prefix annotations for one statement-sequence expression.
fn write_statement_sequence_expression_prefix<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
    start_offset: Option<u32>,
) -> FormatResult<()> {
    if matches!(expression, Expression::Declaration(_)) {
        return Ok(());
    }

    if expression_is_lambda_declaration(f.context(), expression_id) {
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
    f: &mut TsppFormatter<'ast, '_>,
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

    let needs_terminator =
        expression_needs_statement_terminator(f.context(), expression, expression_is_value_tail);

    // write the statement terminator and its comments
    if needs_terminator {
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
    // preserve comments owned by a semicolonless statement
    else {
        write_semicolonless_statement_comments(f, expression_id, following_expression_start)?;
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

    matches!(expression, Expression::ObjectExpression { .. })
}

/// Format a block inline with zero or one expression (including label and infix annotations).
/// Format block contents with compact inner spacing.
#[inline]
pub(crate) fn format_block_body_narrow<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    debug_assert!(block.len() <= 1);

    // body
    if block.is_empty() {
        write!(f, [token("{"), token("}")])?;
    } else {
        let expression_id = block.first_expression().ok_or(FormatError::SyntaxError {
            message: "nonempty narrow block requires one expression",
        })?;
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
    f: &mut TsppFormatter<'ast, '_>,
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
                |f: &mut TsppFormatter<'ast, '_>| {
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
                |f: &mut TsppFormatter<'ast, '_>| {
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

/// The enclosing form of one statement sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatementSequenceKind {
    /// A source file program.
    Program,
    /// A parsed block body.
    Block,
    /// A statement slice without an enclosing source node.
    Detached,
}

impl StatementSequenceKind {
    /// Return whether imports at the sequence head should be organized.
    const fn organizes_imports(self) -> bool {
        matches!(self, Self::Program | Self::Block)
    }

    /// Return whether comments before the first expression belong to the sequence.
    const fn writes_file_prefix(self) -> bool {
        matches!(self, Self::Program)
    }

    /// Return whether every statement needs an initial hard break.
    const fn always_breaks_statements(self) -> bool {
        matches!(self, Self::Program)
    }
}

/// The source positions and enclosing form of one statement sequence.
#[derive(Debug, Clone, Copy)]
struct StatementSequence {
    /// The enclosing form.
    kind: StatementSequenceKind,
    /// The expression that supplies the sequence value.
    tail_expression: Option<LocalNodeId<Expression>>,
    /// The source position immediately before the first expression.
    source_start: Option<(FileId, u32)>,
    /// The final leading comment already emitted by the enclosing block.
    first_prefix_comment_end: Option<u32>,
}

impl StatementSequence {
    /// Create one program statement sequence.
    const fn program() -> Self {
        Self {
            kind: StatementSequenceKind::Program,
            tail_expression: None,
            source_start: None,
            first_prefix_comment_end: None,
        }
    }

    /// Create one block statement sequence.
    fn block(
        context: &TsppFormatContext<'_>,
        block_id: LocalNodeId<Block>,
        allow_value_tail: bool,
        first_prefix_comment_end: Option<u32>,
    ) -> Self {
        let block = context.tree.get(block_id);
        let tail_expression = if allow_value_tail {
            block.tail_expression
        } else {
            None
        };
        let block_span = context.span(block_id);
        let source_start = context
            .first_token_in_span(block_span)
            .map(|token| (token.span.file, token.span.end));

        Self {
            kind: StatementSequenceKind::Block,
            tail_expression,
            source_start,
            first_prefix_comment_end,
        }
    }

    /// Create one detached statement sequence.
    fn detached(expressions: &[LocalNodeId<Expression>], allow_value_tail: bool) -> Self {
        let tail_expression = if allow_value_tail {
            expressions.last().copied()
        } else {
            None
        };

        Self {
            kind: StatementSequenceKind::Detached,
            tail_expression,
            source_start: None,
            first_prefix_comment_end: None,
        }
    }
}

/// Collect ignore ranges for one statement sequence.
fn statement_ignore_ranges(
    context: &TsppFormatContext<'_>,
    expressions: &[LocalNodeId<Expression>],
) -> HashMap<u32, Span> {
    if !context.has_ignore_directive_markers() {
        return HashMap::new();
    }

    ignore_ranges_for_nodes(context, expressions, context.source_comments())
}

/// Organize the import section at the head of one statement sequence.
fn organize_statement_imports<'a>(
    context: &TsppFormatContext<'_>,
    expressions: &'a [LocalNodeId<Expression>],
    sequence: StatementSequence,
    has_ignore_ranges: bool,
) -> Cow<'a, [LocalNodeId<Expression>]> {
    if !sequence.kind.organizes_imports()
        || !context.options.organize_imports.is_enabled()
        || has_ignore_ranges
    {
        return Cow::Borrowed(expressions);
    }

    let import_count = expressions
        .iter()
        .take_while(|&&expression_id| is_import(expression_id, context.tree))
        .count();
    if import_count <= 1 {
        return Cow::Borrowed(expressions);
    }

    let mut organized = sort_imports(&expressions[..import_count], context.tree, context.strings);
    organized.extend_from_slice(&expressions[import_count..]);

    Cow::Owned(organized)
}

/// Return whether a blank line belongs before one statement.
fn statement_has_blank_line_before(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    expression_span: Span,
    ignore_range: Option<Span>,
    previous_expression_id: LocalNodeId<Expression>,
    previous_output_end: Option<(FileId, u32)>,
) -> bool {
    let Some(ignore_range) = ignore_range else {
        return expression_has_lines_before(context, expression_id);
    };

    let previous_end = previous_output_end
        .filter(|(file, _)| *file == expression_span.file)
        .map(|(_, end)| end)
        .or_else(|| {
            let previous_span = context.span(previous_expression_id);
            (previous_span.file == expression_span.file).then_some(previous_span.end)
        });
    let Some(previous_end) = previous_end else {
        return false;
    };

    has_blank_line_between_offsets(
        context,
        expression_span.file,
        previous_end,
        ignore_range.start,
    )
}

/// Format one statement sequence with shared spacing, import, and ignore handling.
fn format_statement_sequence<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expressions: &[LocalNodeId<Expression>],
    sequence: StatementSequence,
) -> FormatResult<()> {
    let ignore_ranges = statement_ignore_ranges(f.context(), expressions);
    let expressions = organize_statement_imports(
        f.context(),
        expressions,
        sequence,
        !ignore_ranges.is_empty(),
    );

    let mut previous_expression_id = None;
    let mut previous_output_end = sequence.source_start;
    let mut previous_output_was_ignored = false;
    let mut previous_import_id = None;
    let mut skip_until = None;

    for (index, expression_id) in expressions.iter().copied().enumerate() {
        let expression = f.context().tree.get(expression_id);
        let expression_span = f.context().span(expression_id);
        let ignore_range = ignore_ranges.get(&expression_id.id).copied();

        // skip statements already covered by one raw ignore range
        if let Some(skip_end) = skip_until {
            if expression_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // preserve statement and import-group spacing
        let is_import = is_import(expression_id, f.context().tree);
        if let Some(previous_expression_id) = previous_expression_id {
            let has_blank_line = statement_has_blank_line_before(
                f.context(),
                expression_id,
                expression_span,
                ignore_range,
                previous_expression_id,
                previous_output_end,
            );

            if ignore_range.is_none() {
                if sequence.kind.always_breaks_statements() || !has_blank_line {
                    write!(f, [hard_line_break()])?;
                }

                let separate_imports = previous_import_id.is_some_and(|previous_import_id| {
                    sequence.kind.organizes_imports()
                        && f.context().options.organize_imports.is_enabled()
                        && is_import
                        && should_insert_blank_between(
                            previous_import_id,
                            expression_id,
                            f.context().tree,
                            f.context().strings,
                        )
                });
                if separate_imports || has_blank_line {
                    write!(f, [empty_line()])?;
                }
            }
        }

        // preserve one ignored source range verbatim
        if let Some(ignore_range) = ignore_range {
            let prefix_start = ignored_range_prefix_start(
                f.context(),
                previous_output_end,
                previous_output_was_ignored,
                previous_expression_id,
                expression_id,
                expression_span,
            );
            if prefix_start < ignore_range.start {
                let prefix_span = Span::new(ignore_range.file, prefix_start, ignore_range.start);
                write_source_span(f, prefix_span)?;
            }

            write_source_span(f, ignore_range)?;
            skip_until = Some(ignore_range.end);
            previous_expression_id = Some(expression_id);
            previous_output_end = Some((ignore_range.file, ignore_range.end));
            previous_output_was_ignored = true;
            previous_import_id = None;
            continue;
        }

        // write comments between the previous output and this statement
        if sequence.kind.writes_file_prefix() && previous_expression_id.is_none() {
            write_expression_gap_comments(f, 0, expression_id)?;
        } else if previous_expression_id.is_some() || previous_output_end.is_some() {
            let gap_start = previous_output_end
                .filter(|(file, _)| *file == expression_span.file)
                .map(|(_, end)| end)
                .or_else(|| {
                    previous_expression_id
                        .map(|previous_expression_id| f.context().span(previous_expression_id).end)
                })
                .unwrap_or(expression_span.start);
            write_expression_gap_comments(f, gap_start, expression_id)?;
        }

        let first_prefix_comment_end = previous_expression_id
            .is_none()
            .then_some(sequence.first_prefix_comment_end)
            .flatten();
        let following_expression_start = expressions
            .get(index + 1)
            .map(|expression_id| expression_following_span_start(f.context(), *expression_id));
        let output_end = format_statement_sequence_expression(
            f,
            expression_id,
            expression,
            node_has_ignore_directive(f.context(), expression_id),
            sequence.tail_expression.is_some(),
            sequence.tail_expression == Some(expression_id),
            first_prefix_comment_end,
            following_expression_start,
        )?;

        f.context_mut()
            .comments_mut()
            .skip_comments_before(output_end);

        previous_expression_id = Some(expression_id);
        previous_output_end = Some((expression_span.file, output_end));
        previous_output_was_ignored = false;
        previous_import_id = is_import.then_some(expression_id);
    }

    Ok(())
}

/// Format one block statement sequence with spacing and ignore handling.
pub(crate) fn format_block_statement_sequence<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expressions: &[LocalNodeId<Expression>],
    allow_value_tail: bool,
) -> FormatResult<()> {
    let sequence = StatementSequence::detached(expressions, allow_value_tail);

    format_statement_sequence(f, expressions, sequence)
}

/// Format one block statement sequence with spacing and ignore handling.
pub(crate) fn format_block_statement_sequence_for_block<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
    allow_value_tail: bool,
    leading_prefix_comment_start: Option<u32>,
) -> FormatResult<()> {
    let expressions = f
        .context()
        .tree
        .get(block_id)
        .iter_expressions()
        .collect::<Vec<_>>();
    let sequence = StatementSequence::block(
        f.context(),
        block_id,
        allow_value_tail,
        leading_prefix_comment_start,
    );

    format_statement_sequence(f, &expressions, sequence)
}

/// Format one program statement sequence with spacing, import organization, and ignore handling.
fn format_program_statement_sequence<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    format_statement_sequence(f, expressions, StatementSequence::program())
}

/// Return true when the final expression in this block is value-position.
pub(crate) fn block_allows_value_tail(
    context: &TsppFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    if block.context != BlockContext::Expression {
        return false;
    }

    let Some((block_expression_id, block_expression_type)) = context.parent(block_id) else {
        return false;
    };
    if block_expression_type == NodeType::MatchArm {
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

    !expression_is_in_statement_context(context, block_expression_id)
}

/// Return true when this expression is the value tail of one block.
pub(crate) fn expression_is_value_block_tail(
    context: &TsppFormatContext<'_>,
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

/// Return true when this expression is formatted in a statement context.
pub(crate) fn expression_is_in_statement_context(
    context: &TsppFormatContext<'_>,
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
                Expression::If { .. } => false,
                Expression::While { body, .. }
                | Expression::ForEach { body, .. }
                | Expression::For { body, .. }
                | Expression::Loop { body, .. } => body.id == expression_id.id,
                Expression::Try { .. } => false,
                _ => false,
            };

            should_inherit_parent_position
                && expression_is_in_statement_context(context, parent_expression_id)
        }
        NodeType::Block => expression_is_in_statement_context_inside_parent_block(
            context,
            LocalNodeId::<Block>::new(parent_id),
            expression_id,
        ),
        NodeType::Declaration => expression_is_in_statement_context_inside_parent_declaration(
            context,
            LocalNodeId::<Declaration>::new(parent_id),
            expression_id,
        ),
        NodeType::Member => expression_is_in_statement_context_inside_parent_member(
            context,
            LocalNodeId::<Member>::new(parent_id),
            expression_id,
        ),
        NodeType::Property => expression_is_in_statement_context_inside_parent_property(
            context,
            LocalNodeId::<Property>::new(parent_id),
            expression_id,
        ),
        _ => false,
    }
}

/// Return true when one block child is formatted in a statement context.
fn expression_is_in_statement_context_inside_parent_block(
    context: &TsppFormatContext<'_>,
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

/// Return true when one declaration child is formatted in a statement context.
fn expression_is_in_statement_context_inside_parent_declaration(
    context: &TsppFormatContext<'_>,
    parent_declaration_id: LocalNodeId<Declaration>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_declaration = context.tree.get(parent_declaration_id);

    match parent_declaration {
        Declaration::Function(function) => function.body.is_some_and(|body_expression_id| {
            if body_expression_id.id != expression_id.id {
                return false;
            }

            function_body_is_statement_context(context, &function.signature)
        }),
        Declaration::Global(global) => global.expressions.contains(&expression_id),
        Declaration::Module(module) => module.expressions.contains(&expression_id),
        _ => false,
    }
}

/// Return true when one member child is formatted in a statement context.
fn expression_is_in_statement_context_inside_parent_member(
    context: &TsppFormatContext<'_>,
    parent_member_id: LocalNodeId<Member>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_member = context.tree.get(parent_member_id);

    match parent_member {
        Member::Method {
            signature, body, ..
        } => body.as_ref().is_some_and(|body_expression_id| {
            body_expression_id.id == expression_id.id
                && function_body_is_statement_context(context, signature)
        }),
        Member::StaticBlock { body, .. } | Member::ConstBlock { body, .. } => {
            body.id == expression_id.id
        }
        _ => false,
    }
}

/// Return true when one property child is formatted in a statement context.
fn expression_is_in_statement_context_inside_parent_property(
    context: &TsppFormatContext<'_>,
    parent_property_id: LocalNodeId<Property>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_property = context.tree.get(parent_property_id);

    match parent_property {
        Property::Method {
            signature, body, ..
        } => body.as_ref().is_some_and(|body_expression_id| {
            body_expression_id.id == expression_id.id
                && function_body_is_statement_context(context, signature)
        }),
        _ => false,
    }
}

/// Return true when one function-like body is formatted in a statement context.
fn function_body_is_statement_context(
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    matches!(
        context.tree.get(type_id),
        TypeExpression::Keyword {
            value: TypeLiteral::Void,
        }
    )
}
