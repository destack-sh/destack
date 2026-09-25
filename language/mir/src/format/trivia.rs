use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::source::TokenType;
use crate::{LocalNodeId, Node, Tree, Writer};

/// One normalized comment line.
#[derive(Debug, Clone)]
struct CommentLine {
    /// The number of blank lines before this comment.
    blank_lines_before: usize,
    /// The exact comment text.
    text: String,
}

/// One normalized comment block.
struct CommentBlock {
    /// The comment lines.
    lines: Vec<CommentLine>,
    /// The blank lines after the final comment.
    trailing_blank_lines: usize,
}

/// Write comment lines between byte offsets as standalone lines.
pub(crate) fn write_comments_before<'a>(
    tree: &Tree,
    start: u32,
    end: u32,
    writer: &mut Writer<'a, '_>,
) -> FormatResult<bool> {
    let comments = collect_comments(tree, start, end);

    write_comment_block(&comments, 0, writer)
}

/// Write comments after one anchor, keeping inline comments inline.
pub(crate) fn write_comments_after<'a>(
    tree: &Tree,
    start: u32,
    end: u32,
    writer: &mut Writer<'a, '_>,
) -> FormatResult<bool> {
    let wrote_inline = write_inline_comment_after(tree, start, end, writer)?;
    let block_start = tree
        .tokens()
        .iter()
        .filter(|token| token.span.end > start)
        .take_while(|token| token.span.start < end)
        .find(|token| token.ty == TokenType::Newline)
        .map(|token| token.span.start);
    let mut wrote_comment = wrote_inline;

    // write subsequent comments as standalone lines
    if let Some(block_start) = block_start
        && write_comments_before(tree, block_start, end, writer)?
    {
        wrote_comment = true;
    }

    Ok(wrote_comment)
}

/// Write comments after one anchor on the same line.
pub(crate) fn write_inline_comment_after<'a>(
    tree: &Tree,
    start: u32,
    end: u32,
    writer: &mut Writer<'a, '_>,
) -> FormatResult<bool> {
    let Some(comment) = tree.inline_comment_between(start, end) else {
        return Ok(false);
    };

    write!(writer, [space(), copied_text(&comment.text)])?;

    Ok(true)
}

/// Write leading comments for one node.
pub(crate) fn write_node_leading_comments<'a, T>(
    tree: &Tree,
    id: LocalNodeId<T>,
    writer: &mut Writer<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some(span) = tree.leading_comment_span(id) else {
        return Ok(false);
    };

    write_comments_before(tree, span.start, span.end, writer)
}

/// Write leading comments after one canonical sibling separator.
pub(crate) fn write_node_leading_comments_after_separator<'a, T>(
    tree: &Tree,
    id: LocalNodeId<T>,
    writer: &mut Writer<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some(span) = tree.leading_comment_span(id) else {
        return Ok(false);
    };
    let comments = collect_comments(tree, span.start, span.end);

    write_comment_block(&comments, 1, writer)
}

/// Write trailing comments after the final top-level item.
pub(crate) fn write_top_level_comments_after<'a, T>(
    tree: &Tree,
    id: LocalNodeId<T>,
    writer: &mut Writer<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some(span) = tree.get_span(id) else {
        return Ok(false);
    };
    let scope_end = tree
        .tokens()
        .last()
        .map(|token| token.span.end)
        .unwrap_or(span.end);

    write!(writer, [hard_line_break()])?;

    let comments = collect_comments(tree, span.end, scope_end);

    write_comment_block(&comments, 1, writer)
}

/// Collect normalized comments between byte offsets.
fn collect_comments(tree: &Tree, start: u32, end: u32) -> CommentBlock {
    let mut lines = Vec::new();
    let mut newline_count = 0usize;

    for token in tree.tokens() {
        if token.span.end <= start {
            continue;
        }
        if token.span.start >= end {
            break;
        }

        match token.ty {
            TokenType::Comment => {
                lines.push(CommentLine {
                    blank_lines_before: newline_count.saturating_sub(1),
                    text: tree.source_text(token.span).to_string(),
                });
                newline_count = 0;
            }
            TokenType::Newline => newline_count += 1,
            TokenType::Whitespace => {}
            _ => {}
        }
    }

    CommentBlock {
        lines,
        trailing_blank_lines: newline_count.saturating_sub(1),
    }
}

/// Write one normalized comment block.
fn write_comment_block<'a>(
    block: &CommentBlock,
    first_blank_lines_to_skip: usize,
    writer: &mut Writer<'a, '_>,
) -> FormatResult<bool> {
    if block.lines.is_empty() {
        return Ok(false);
    }

    // write each comment with its normalized leading gap
    for (index, comment) in block.lines.iter().enumerate() {
        let blank_lines = if index == 0 {
            comment
                .blank_lines_before
                .saturating_sub(first_blank_lines_to_skip)
        } else {
            comment.blank_lines_before
        };

        if index > 0 && blank_lines == 0 {
            write!(writer, [hard_line_break()])?;
        } else {
            write_blank_lines(blank_lines, writer)?;
        }

        write!(writer, [copied_text(&comment.text)])?;
    }

    // preserve the normalized trailing gap
    if block.trailing_blank_lines > 0 {
        write_blank_lines(block.trailing_blank_lines, writer)?;
    } else {
        write!(writer, [hard_line_break()])?;
    }

    Ok(true)
}

/// Write one normalized blank-line gap.
fn write_blank_lines<'a>(count: usize, writer: &mut Writer<'a, '_>) -> FormatResult<()> {
    for _ in 0..count {
        write!(writer, [empty_line()])?;
    }

    Ok(())
}
